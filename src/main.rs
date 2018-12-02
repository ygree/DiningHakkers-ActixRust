extern crate actix;
extern crate futures;
extern crate rand;

use actix::dev::*;
// use actix::prelude::*;
// use futures::{future, Future};
use std::time::Duration;
// use std::fmt::Error;

fn five_seconds() -> Duration {
//    Duration::new(5u64 + rand::random::<u64>() % 5, 0)
    Duration::new(5, 0)
//    Duration::new(0, 500_000_000)
//    Duration::new(5, 0) / 10
}

fn ten_seconds() -> Duration {
//    Duration::new(10u64 + rand::random::<u64>() % 10, 0)
    Duration::new(10, 0)
//    Duration::new(0, 1_000_000_000)
//    Duration::new(10, 0) / 10
}

#[derive(Debug)]
enum Chopstick {
    Available,
    TakenBy(Addr<Hakker>),
}

/// Chopstick is an actor, it can be taken, and put back
///
impl Actor for Chopstick {
    type Context = Context<Self>;
}

enum ChopstickMessage {
    Take(Addr<Hakker>),
    Put(Addr<Hakker>),
}

#[derive(Debug)]
enum ChopstickAnswer {
    Taken(Addr<Chopstick>),
    Busy,
    PutBack,
}

impl Handler<ChopstickMessage> for Chopstick {
    type Result = ChopstickAnswer;

    fn handle(&mut self, msg: ChopstickMessage, ctx: &mut Self::Context) -> Self::Result {
        let (new_state, result) = match self {
            // When a Chopstick is taken by a hakker
            // It will refuse to be taken by other hakkers
            // But the owning hakker can put it back
            Chopstick::TakenBy(ref hakker) => match msg {
                ChopstickMessage::Take(_) => (None, ChopstickAnswer::Busy),
                ChopstickMessage::Put(ref sender) if sender == hakker => {
                    (Some(Chopstick::Available), ChopstickAnswer::PutBack)
                }
                _ => unreachable!("Chopstick can't be put back by another hakker"),
            },
            // When a Chopstick is available, it can be taken by a hakker
            Chopstick::Available => match msg {
                ChopstickMessage::Take(hakker) => (
                    Some(Chopstick::TakenBy(hakker)),
                    ChopstickAnswer::Taken(ctx.address()),
                ),
                _ => unreachable!("Chopstick isn't taken"),
            },
        };
        if let Some(ns) = new_state {
            *self = ns;
        }
        result
    }
}

impl Message for ChopstickMessage {
    type Result = ChopstickAnswer;
}

impl<A, M> MessageResponse<A, M> for ChopstickAnswer
where
    A: Actor,
    M: Message<Result = ChopstickAnswer>,
{
    fn handle<R: ResponseChannel<M>>(self, ctx: &mut <A as Actor>::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

#[derive(Debug)]
struct Hakker {
    name: String,
    left: Addr<Chopstick>,
    right: Addr<Chopstick>,
    state: HakkerState,
}

#[derive(Debug)]
enum HakkerState {
    Waiting,
    Thinking,
    Hungry,
    WaitingForOtherChopstick {
        waiting_on: Addr<Chopstick>,
        taken: Addr<Chopstick>,
    },
    Eating,
    FirstChopstickDenied,
}

impl Actor for Hakker {
    type Context = Context<Self>;
}

enum HakkerMessage {
    Eat,
    Think,
}

impl Handler<ChopstickAnswer> for Hakker {
    type Result = ();

    fn handle(&mut self, msg: ChopstickAnswer, ctx: &mut Self::Context) -> Self::Result {
        let new_state = match self.state {
            HakkerState::Hungry => match msg {
                ChopstickAnswer::Taken(chopstick) => {
                    let waiting_on = if self.left == chopstick {
                        self.right.clone()
                    } else if self.right == chopstick {
                        self.left.clone()
                    } else {
                        unreachable!("Received unknown chopstick: {:?}", chopstick)
                    };
                    // println!("taken first chopstick: {:?}", chopstick);
                    Some(HakkerState::WaitingForOtherChopstick {
                        waiting_on,
                        taken: chopstick,
                    })
                }
                ChopstickAnswer::Busy => Some(HakkerState::FirstChopstickDenied),
                _ => unreachable!("Unexpected message in state Hungry"),
            },
            // When a hakker is waiting for the last chopstick it can either obtain it
            // and start eating, or the other chopstick was busy, and the hakker goes
            // back to think about how he should obtain his chopsticks :-)
            HakkerState::WaitingForOtherChopstick {
                ref waiting_on,
                ref taken,
            } => match msg {
                ChopstickAnswer::Taken(ref chopstick) if waiting_on == chopstick => {
                    println!(
                        "{} has picked up <chopstick> and <chopstick> and starts to eat",
                        self.name
                    );
                    // println!(
                    //     "{} has picked up {:?} and {:?} and starts to eat",
                    //     self.name, self.left, self.right
                    // );

                    ctx.notify_later(HakkerMessage::Think, five_seconds());

                    Some(HakkerState::Eating)
                }
                ChopstickAnswer::Busy => {
                    taken.do_send(ChopstickMessage::Put(ctx.address()));

                    ctx.notify_later(HakkerMessage::Eat, ten_seconds());
                    Some(HakkerState::Thinking)
                }
                _ => unreachable!("Unexpected message in state WaitingForOtherChopstick"),
            },
            // When the results of the other grab comes back,
            // he needs to put it back if he got the other one.
            // Then go back and think and try to grab the chopsticks again
            HakkerState::FirstChopstickDenied => match msg {
                ChopstickAnswer::Busy => {
                    ctx.notify_later(HakkerMessage::Eat, ten_seconds());
                    Some(HakkerState::Thinking)
                }
                ChopstickAnswer::Taken(chopstick) => {
                    chopstick.do_send(ChopstickMessage::Put(ctx.address()));

                    ctx.notify_later(HakkerMessage::Eat, ten_seconds());
                    Some(HakkerState::Thinking)
                }
                _ => unreachable!("Unexpected message in state FirstChopstickDenied"),
            },
            _ => unimplemented!("TODO"),
        };
        if let Some(ns) = new_state {
            self.state = ns;
        }
    }
}

impl Message for ChopstickAnswer {
    type Result = ();
}

impl Handler<HakkerMessage> for Hakker {
    type Result = ();

    fn handle(&mut self, msg: HakkerMessage, ctx: &mut Self::Context) -> Self::Result {
        let (new_state, resp) = match self.state {
            HakkerState::Waiting => match msg {
                HakkerMessage::Think => {
                    println!("{} starts to think", self.name);
                    ctx.notify_later(HakkerMessage::Eat, five_seconds());
                    (Some(HakkerState::Thinking), ())
                }
                _ => unreachable!("When waiting state Hakker can only start thinking."),
            },
            // When a hakker is thinking it can become hungry
            // and try to pick up its chopsticks and eat
            HakkerState::Thinking => match msg {
                HakkerMessage::Eat => {
                    self.left
                        .send(ChopstickMessage::Take(ctx.address()))
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(m) => {
                                    // println!("getting response from the left chopstick: {:?}", m);
                                    ctx.address()
                                        .send(m)
                                        .into_actor(act)
                                        .then(|r, a, c| actix::fut::ok(()))
                                }
                                _ => unimplemented!(), // actix::fut::ok(()) //ignore
                            }
                        }).spawn(ctx);
                    self.right
                        .send(ChopstickMessage::Take(ctx.address()))
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(m) => {
                                    // println!("getting response from the right chopstick: {:?}", m);
                                    ctx.address()
                                        .send(m)
                                        .into_actor(act)
                                        .then(|r, a, c| actix::fut::ok(()))
                                }
                                _ => unimplemented!(), // actix::fut::ok(()) //ignore
                            }
                        }).spawn(ctx);

                    (Some(HakkerState::Hungry), ())
                }
                _ => unreachable!("When thinking hakker can only start eating, not thinking!"),
            },
            // When a hakker is eating, he can decide to start to think,
            // then he puts down his chopsticks and starts to think
            HakkerState::Eating => match msg {
                HakkerMessage::Think => {
                    println!("{} puts down his chopsticks and starts to think", self.name);

                    self.left.do_send(ChopstickMessage::Put(ctx.address())); //TODO: is do_send the best option here? is it blocking?
                    self.right.do_send(ChopstickMessage::Put(ctx.address()));

                    ctx.notify_later(HakkerMessage::Eat, five_seconds());
                    (Some(HakkerState::Thinking), ())
                }
                HakkerMessage::Eat => {
                    unreachable!("When eating hakker can only start thinking, not eating!")
                }
            },

            _ => unreachable!("Unexpected state"),
        };
        if let Some(ns) = new_state {
            self.state = ns;
        }
        resp
    }
}

impl Message for HakkerMessage {
    type Result = (); //TODO
}

fn main() {
    let system = actix::System::new("test");

    let number_of_chopstick = 20000;

    let chopsticks: Vec<_> = (1..=number_of_chopstick).map(|_| Chopstick::Available.start()).collect();

//    let hakkers = ["Ghosh", "Boner", "Klang", "Krasser", "Manie"];
    let hakkers = (1..=number_of_chopstick).map(|i| format!("hakker-{}", i)).collect::<Vec<_>>();

    for i in 0..number_of_chopstick {
        let hakker = Hakker {
            name: hakkers[i].clone(),
            left: chopsticks[i].clone(),
            right: chopsticks[(i + 1) % number_of_chopstick].clone(),
            state: HakkerState::Waiting,
        }.start();

        hakker.do_send(HakkerMessage::Think);
    }

    system.run();
}
