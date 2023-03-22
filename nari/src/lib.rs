//! Nari is a crate aimed to provide different productivity tools to your application.
//!
//! It is built with consistency between runs in mind, to achieve this it saves most
//! of its information in the filesystem, different approaches like using NoSQL/SQL
//! databases or an entirely in-memory approach may come in the future.
//!   
//! [`Event`] represents any possible event that can happen. It provides any possible
//! important information that a event can have, check its documentation for further
//! information.
//!
//! [`Database`] represents a database, it provides functions to interact with it and
//! it lays mostly inside files. These files althought human readable([`.ron`]), are not supposed to
//! be interacted outside of nari. We may change the file specification or make nari work
//! better if these files are modified externally in the future.
//!
//! [`EventListener`] provides an easy way to create a connection using tokio channels to
//! future events. It sends events through a mspc channel whenever their unix timestamp is
//! reached, how often this condition is checked  
//!
//! To see it in action you can look at [`examples`] to get a quick grasp on how to get running with nari.
//!
//! If you would rather have a fully fledged application ready, you can check our [`github repo`]
//! for our projects built from nari.
//!
//! [`Event`]: crate::models::event::Event
//! [`Database`]: crate::models::Database
//! [`EventListener`]: crate::models::event::EventListener
//! [`.ron`]: https://github.com/ron-rs/ron
//! [`github repo`]: https://github.com/HiccupEnthusiast/Nari
//! [`examples`]: https://github.com/HiccupEnthusiast/Nari/nari/examples

/// This module holds the structure of nari.
pub mod models;
