extern crate actix;
extern crate futures;

use actix::dev::*;
// use actix::prelude::*;
use futures::{future, Future};
use std::time::Duration;
// use std::fmt::Error;

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
    Taken,
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
                ChopstickMessage::Take(hakker) => {
                    (Some(Chopstick::TakenBy(hakker)), ChopstickAnswer::Taken)
                }
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

struct Hakker {
    name: String,
    left: Addr<Chopstick>,
    right: Addr<Chopstick>,
    state: HakkerState,
}

enum HakkerState {
    Waiting,
    Thinking,
    Hungry,
    WaitingForOtherChopstick(Addr<Chopstick>),
}

impl Actor for Hakker {
    type Context = Context<Self>;
}

enum HakkerMessage {
    Eat,
    Think,
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
                            println!("getting response from the left chopstick: {:?}", res);
                            //TODO can change state right here
                            // act.state = HakkerState::WaitingForOtherChopstick(act.left.clone());

                            //TODO can send a message to itself
                            // ctx.address()
                            //     .send(HakkerMessage::Eat)
                            //     .into_actor(act)
                            //     .then(|r, a, c| actix::fut::ok(()))

                            actix::fut::ok(())
                        }).spawn(ctx);

                    (Some(HakkerState::Hungry), ())
                }
                _ => unimplemented!("In thinking mode it can only handle Eat messag"),
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
