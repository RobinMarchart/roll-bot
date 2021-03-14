use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_xoshiro::Xoshiro256PlusPlus;
use robins_dice_roll::{
    dice_roll::{EvaluationErrors, ExpressionEvaluate},
    dice_types::Expression,
};
use std::{
    borrow::Borrow,
    result::Result,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::spawn,
    time::{interval, sleep, sleep_until, Instant},
};

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

async fn start_rng_provider(rng_reseed: Duration) -> mpsc::Sender<RngProviderOps> {
    let (sender, receiver) = mpsc::channel(32);
    spawn(async move {
        RngProvider {
            rng: ChaCha20Rng::from_entropy(),
            receiver,
        }
        .run()
        .await
    });
    let sender_clone = sender.clone();
    spawn(async move {
        sleep(rng_reseed.clone()).await;
        let mut interval = interval(rng_reseed);
        loop {
            interval.tick().await;
            sender_clone
                .send(RngProviderOps::SetCryptoRng(ChaCha20Rng::from_entropy()))
                .await
                .unwrap();
        }
    });
    sender
}

pub struct RollExecutor {
    pool: ThreadPool,
    timeout: Duration,
    rng_gen: mpsc::Sender<RngProviderOps>,
}
impl RollExecutor {
    pub async fn new(size: u32, timeout: Duration, rng_reseed: Duration) -> RollExecutor {
        let rng = start_rng_provider(rng_reseed).await;
        RollExecutor {
            pool: Builder::new()
                .core_size(1)
                .max_size(size)
                .name("Roll Worker".to_string())
                .build(),
            timeout,
            rng_gen: rng,
        }
    }

    pub async fn roll<Expr>(&self, expr: Expr) -> Result<Vec<(i64, Vec<i64>)>, EvaluationErrors>
    where
        Expr: Borrow<Expression> + Sized + Send + 'static,
    {
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
            result_sender
                .send(expr.borrow().evaluate(
                    &mut move || timeout_signal.load(std::sync::atomic::Ordering::Relaxed),
                    &mut rng,
                ))
                .unwrap();
        });
        let timeout_clone = self.timeout.clone();
        spawn(async move {
            sleep_until(time_receiver.await.unwrap() + timeout_clone).await;
            timeout_signal_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        result_receiver.await.unwrap()
    }
}
