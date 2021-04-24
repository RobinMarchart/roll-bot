use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_xoshiro::Xoshiro256PlusPlus;
use robins_dice_roll::{
    dice_roll::{EvaluationErrors, ExpressionEvaluate},
    LabeledExpression,
};
use std::{
    borrow::Borrow,
    fmt::format,
    result::Result,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::spawn,
    time::{interval, sleep, sleep_until, Instant},
};

use crate::bot_manager::StopListener;
use rusty_pool::{Builder, ThreadPool};

#[derive(Debug)]
enum RngProviderOps {
    GetRng(oneshot::Sender<Xoshiro256PlusPlus>),
    SetCryptoRng(ChaCha20Rng),
}

struct RngProvider {
    rng: ChaCha20Rng,
    receiver: mpsc::Receiver<RngProviderOps>,
}

impl RngProvider {
    pub async fn run(&mut self) {
        loop {
            match self.receiver.recv().await {
                Some(op) => match op {
                    RngProviderOps::GetRng(channel) => {
                        let mut seed: <Xoshiro256PlusPlus as SeedableRng>::Seed =
                            Default::default();
                        self.rng.fill(&mut seed);
                        channel.send(Xoshiro256PlusPlus::from_seed(seed)).unwrap()
                    }
                    RngProviderOps::SetCryptoRng(rng) => self.rng = rng,
                },
                None => {
                    break;
                }
            }
        }
    }
}

async fn start_rng_provider<Stop: StopListener>(
    rng_reseed: Duration,
    mut stop: Stop,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<RngProviderOps>) {
    let (sender, receiver) = mpsc::channel(32);
    let rng_handle = spawn(async move {
        RngProvider {
            rng: ChaCha20Rng::from_entropy(),
            receiver,
        }
        .run()
        .await
    });
    let sender_clone = sender.clone();
    (
        spawn(async move {
            tokio::select! {
                _ = sleep(rng_reseed.clone())=>{

                }
                _ = stop.wait_stop()=>{
                    drop(sender_clone);
                    log::info!("stopped reseeding task");
                    return rng_handle.await.unwrap();
                }
            };
            let mut interval = interval(rng_reseed);
            loop {
                interval.tick().await;
                tokio::select! {
                    _ = interval.tick()=>{
                        match sender_clone
                    .send(RngProviderOps::SetCryptoRng(ChaCha20Rng::from_entropy()))
                    .await
                {
                    Ok(_) => {}
                    Err(_) => {
                        break;
                    }
                }

                    }
                    _ = stop.wait_stop()=>{break;}
                }
            }
            drop(sender_clone);
            log::info!("stopped reseeding task");
            rng_handle.await.unwrap()
        }),
        sender,
    )
}

pub struct RollExecutor {
    pool: ThreadPool,
    timeout: Duration,
    rng_gen: mpsc::Sender<RngProviderOps>,
}
impl RollExecutor {
    pub async fn new<Stop: StopListener>(
        size: u32,
        timeout: Duration,
        rng_reseed: Duration,
        stop: Stop,
    ) -> (tokio::task::JoinHandle<()>, RollExecutor) {
        let (handle, rng) = start_rng_provider(rng_reseed, stop).await;
        (
            handle,
            RollExecutor {
                pool: Builder::new()
                    .core_size(1)
                    .max_size(size)
                    .name("Roll Worker".to_string())
                    .build(),
                timeout,
                rng_gen: rng,
            },
        )
    }

    pub async fn roll<Expr>(&self, expr: Expr) -> super::RollExprResult
    where
        Expr: Borrow<super::VersionedRollExpr> + Sized + Send + 'static,
    {
        let text = format!("{}", expr.borrow());
        let (result_sender, result_receiver) = oneshot::channel();
        let (time_sender, time_receiver) = oneshot::channel();
        let timeout_signal = Arc::new(AtomicBool::new(false));
        let timeout_signal_clone = timeout_signal.clone();
        let (rng_send, rng_receive) = oneshot::channel();
        self.rng_gen
            .send(RngProviderOps::GetRng(rng_send))
            .await
            .unwrap();
        let rng = rng_receive.await.unwrap();
        self.pool.execute(move || {
            time_sender.send(Instant::now()).unwrap();
            let mut rng = rng;
            result_sender.send(match expr.borrow() {
                super::VersionedRollExpr::V1(e) => super::RollExprResult {
                    roll: e.evaluate(
                        &mut move || timeout_signal.load(std::sync::atomic::Ordering::Relaxed),
                        &mut rng,
                    ),
                    text,
                    label: None,
                },
                super::VersionedRollExpr::V2(LabeledExpression::Unlabeled(e)) => {
                    super::RollExprResult {
                        roll: e.evaluate(
                            &mut move || timeout_signal.load(std::sync::atomic::Ordering::Relaxed),
                            &mut rng,
                        ),
                        text,
                        label: None,
                    }
                }
                super::VersionedRollExpr::V2(LabeledExpression::Labeled(e, l)) => {
                    super::RollExprResult {
                        roll: e.evaluate(
                            &mut move || timeout_signal.load(std::sync::atomic::Ordering::Relaxed),
                            &mut rng,
                        ),
                        text,
                        label: Some(l.to_owned()),
                    }
                }
            });
        });
        let timeout_clone = self.timeout.clone();
        spawn(async move {
            sleep_until(time_receiver.await.unwrap() + timeout_clone).await;
            timeout_signal_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        result_receiver.await.unwrap()
    }
}
