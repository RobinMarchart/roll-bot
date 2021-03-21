-- Your SQL goes here
create table client_config(
id text primary key not null,
command_prefix text not null default "rrb!",
roll_prefix text not null default "[]",
aliases text not null default "{}",
roll_info boolean not null default true
)
