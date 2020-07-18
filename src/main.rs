#![feature(proc_macro_hygiene, decl_macro)] 

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
use rocket::Rocket; 
extern crate serde;
extern crate google_signin;
extern crate dotenv;
extern crate rocket_cors;
use rocket_cors::{Error};

use dotenv::dotenv;

use rocket_contrib::databases::redis::{self};

mod lib;

#[cfg(test)] mod tests;

#[database("cross_db")]
pub struct DbConn(redis::Connection);

fn rocket() -> Rocket {
	rocket::ignite()
		.attach(DbConn::fairing())
		.mount("/", routes![
							lib::api::subscriber_register,
							lib::api::get_class,
							lib::api::post_class,
							lib::api::get_friends,
							lib::api::post_friend,
							lib::api::update_friend,
							lib::api::delete_friend,
							lib::api::check_friend,
							lib::api::get_profile,
							lib::api::get_other_profile,
							lib::api::post_profile,
							])
}

fn main() -> Result<(), Error> {
	dotenv().ok();
	simple_logger::init().unwrap();
	let cors = rocket_cors::CorsOptions::default().to_cors()?;
	rocket().attach(cors).launch();
	Ok(())
}