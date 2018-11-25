extern crate actix;
extern crate futures;

use actix::dev::MessageResponse;
use actix::dev::ResponseChannel;
use actix::{msgs, Actor, Addr, Arbiter, Context, Handler, Message, Recipient, System};
use futures::{future, Future};

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
                _ => unimplemented!("Can't be put back by another hakker"),
            },
            // When a Chopstick is available, it can be taken by a hakker
            Chopstick::Available => match msg {
                ChopstickMessage::Take(hakker) => {
                    (Some(Chopstick::TakenBy(hakker)), ChopstickAnswer::Taken)
                }
                _ => unimplemented!("It's not taken"),
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

struct Hakker;

impl Actor for Hakker {
    type Context = Context<Self>;
}

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

    let chopstick = Chopstick::Available.start();
    let hakker = Hakker.start();

    // let resp = chopstick.send(ChopstickMessage::Put(hakker));
    let resp = chopstick.send(ChopstickMessage::Take(hakker));

    Arbiter::spawn(resp.then(|resp| {
        match resp {
            Ok(r) => println!("resp: {:?}", r),
            _ => println!("error")
        }
        
        System::current().stop();
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
