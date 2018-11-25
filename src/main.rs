extern crate actix;
extern crate futures;

use actix::dev::*;
// use actix::prelude::*;
use futures::{future, Future};
use std::time::Duration;
// use std::fmt::Error;

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
    WaitingForOtherChopstick(Addr<Chopstick>),
    Eating,
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
        let new_state = match msg {
            ChopstickAnswer::Taken(chopstick) => match self.state {
                HakkerState::Hungry => {
                    let waiting_on = if self.left == chopstick {
                        self.right.clone()
                    } else if self.right == chopstick {
                        self.left.clone()
                    } else {
                        unreachable!("Received unknown chopstick: {:?}", chopstick)
                    };
                    println!("taken first chopstick: {:?}", chopstick);
                    Some(HakkerState::WaitingForOtherChopstick(waiting_on))
                },
                HakkerState::WaitingForOtherChopstick(ref waiting_on) if waiting_on == &chopstick => {
                    println!("{} has picked up {:?} and {:?} and starts to eat", self.name, self.left, self.right);

                    let five_seconds = Duration::new(5, 0);
                    ctx.notify_later(HakkerMessage::Think, five_seconds);

                    Some(HakkerState::Eating)
                }
                _ => unimplemented!("TODO"),
            },
            ChopstickAnswer::Busy => unimplemented!("TODO"),
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
                    let five_seconds = Duration::new(5, 0);
                    ctx.notify_later(HakkerMessage::Eat, five_seconds);
                    (Some(HakkerState::Thinking), ())
                }
                _ => unreachable!("From waiting state Hakker can only start to think."),
            },
            // When a hakker is thinking it can become hungry
            // and try to pick up its chopsticks and eat
            HakkerState::Thinking => match msg {
                HakkerMessage::Eat => {
                    println!("start eating");
                    self.left
                        .send(ChopstickMessage::Take(ctx.address()))
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(m) => {
                                    println!("getting response from the left chopstick: {:?}", m);
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
                                    println!("getting response from the right chopstick: {:?}", m);
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
                _ => unimplemented!("In thinking mode it can only handle Eat messag"),
            },

            HakkerState::Eating => {
                unimplemented!("TODO: eating when receive Think put back chopsticks and start thinking")
            },

            _ => unimplemented!(), //TODO
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

// impl<A, M> MessageResponse<A, M> for ChopstickAnswer
// where
//     A: Actor,
//     M: Message<Result = ChopstickAnswer>,
// {
//     fn handle<R: ResponseChannel<M>>(self, ctx: &mut <A as Actor>::Context, tx: Option<R>) {
//         if let Some(tx) = tx {
//             tx.send(self);
//         }
//     }
// }

// struct Ping;

// #[derive(Debug)]
// struct Pong;

// impl Message for Ping {
//     type Result = Pong;
// }

// struct Summator(usize);

// impl Actor for Summator {
//     type Context = Context<Self>;
// }

// impl <A, M> MessageResponse<A, M> for Pong
// where
//     A: Actor,
//     M: Message<Result = Pong>,
// {
//     fn handle<R: ResponseChannel<M>>(self, ctx: &mut <A as Actor>::Context, tx: Option<R>) {
//         if let Some(tx) = tx {
//             tx.send(self);
//         }
//     }
// }

// impl Handler<Ping> for Summator {
//     type Result = Pong; //TODO how it knows it should be Pong? How does it see Message::Result for Ping

//     fn handle(&mut self, msg: Ping, ctx: &mut Context<Self>) -> Self::Result {
//         self.0 += 1;
//         Pong
//     }
// }

fn main() {
    let system = actix::System::new("test");

    let chopstick1 = Chopstick::Available.start();
    let chopstick2 = Chopstick::Available.start();
    let hakker = Hakker {
        name: "Yury".to_owned(),
        left: chopstick1.clone(),
        right: chopstick2.clone(),
        state: HakkerState::Waiting,
    }.start();

    let req = hakker.send(HakkerMessage::Think);

    // let resp = chopstick.send(ChopstickMessage::Put(hakker));
    // let resp = chopstick1.send(ChopstickMessage::Take(hakker.clone()));

    Arbiter::spawn(req.then(|resp| {
        match resp {
            Ok(r) => println!("resp: {:?}", r),
            _ => println!("error"),
        }

        // System::current().stop();
        future::result(Ok(()))
    }));

    // let addr = Summator(0).start();

    // let res = addr.send(Ping);

    // Arbiter::spawn(res.then(|r| {
    //     match r {
    //         Ok(result) => println!("SUM: {:?}", result),
    //         _ => println!("Something went wrong!"),
    //     }

    //     System::current().stop();
    //     future::result(Ok(()))
    // }));

    system.run();
}
