// use crate::client::SendMyPkg;
// use crate::client::{Client, Connected, HandlerState};
// use crate::protocol::MyPkg;
// use futures::future::BoxFuture;
// use futures::FutureExt;
// use std::fs::File;
// use std::pin::Pin;
// use std::sync::Arc;
// use tokio::runtime::Runtime;

// pub fn wrapped_start<'a>(
//     name: String,
//     file: Vec<String>,
// ) -> Pin<Box<dyn Fn(&mut Client<Connected>) -> BoxFuture<'a, Option<HandlerState<'a>>>>> {
//     // let p = MyPkg::new(name, file).unwrap();
//     let x = name.to_owned();
//     Pin::new(Box::new(move |session: &mut Client<Connected>| {
//         println!("{}", x.to_owned());
//         // let p = p.clone();
//         // let p = MyPkg::new("trash".into(), vec!["Cargo.lock".into(), "Cargo.toml".into()]).unwrap();
//         Box::pin(async {
//             // session.send(p).await.unwrap();
//             Some(HandlerState(start))
//         })
//     }))
// }

// pub fn start<'a>(session: &Client<SendMyPkg>) -> BoxFuture<'a, Option<HandlerState<'a>>> {
//     println!("start");
//     let rt = Runtime::new().unwrap();

//     rt.block_on(async {
//         dbg!(session.mypkg());
//     });
//     Box::pin(async {
//         // let p = MyPkg::new(session.mypkg.namesession).unwrap();
//         // dbg!(session.mypkg());
//         // session.send(session.mypkg()).await.unwrap();
//         // Some(HandlerState(middle))
//         None
//     })
// }

// fn middle<'a>(session: &Client<SendMyPkg>) -> BoxFuture<'a, Option<HandlerState<'a>>> {
//     println!("middle");
//     // session.data = "middle rulez".into();
//     Box::pin(async { Some(HandlerState(end)) })
// }

// fn end<'a>(session: &Client<SendMyPkg>) -> BoxFuture<'a, Option<HandlerState<'a>>> {
//     println!("end");
//     Box::pin(async { None })
// }

// pub fn start<'a>(session: &mut Client<Connected>) -> BoxFuture<'a, Option<HandlerState<'a>>> {
//     println!("start");
//     // let p = protocol::MyPkg::new(name, file).unwrap();
//     // session.conn.send(p).await.unwrap();
//     Box::pin(async { Some(HandlerState(middle)) })
// }
