use crate::DbConn;  
use rocket_contrib::databases::redis::{ Commands, pipe, PipelineCommands};
use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use nanoid::nanoid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Subscriber {
	pub id: Option<String>,
	pub social_id: Option<String>,
	pub proto: Option<String>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Friend {
	pub id: String,
	pub name: String
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
	pub name: String
}

#[derive(Debug, Clone)]
pub struct SubscriberNotFoundError;

impl fmt::Display for SubscriberNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Subscriber was not found in DB")
    }
}

impl error::Error for SubscriberNotFoundError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub struct SubscriberError {
    pub details: String
}

impl SubscriberError {
    fn new(msg: &str) -> SubscriberError {
        SubscriberError{details: msg.to_string()}
    }
}

impl fmt::Display for SubscriberError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl error::Error for SubscriberError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl Subscriber {

	/*
	 * Gets subscriber from proto and token
	 */
	pub fn get_instance_from_social_id(redis: &DbConn, proto: &String, social_id: &String) -> Option<Subscriber> {
		let value = &[proto.to_owned(), ":".to_string(), social_id.to_owned()].concat();
		match redis.hget("tokenmap", value) {
			Ok(value) => return Some(Subscriber {
										id:Some(value),
										social_id:Some(social_id.to_owned()),
										proto:Some(proto.to_owned())
									}),
			Err(_e) => return None 
		};
	}

	/*
	 * Checks if a subscriber exists by id
	 */ 
	pub fn check_subscriber(&mut self, redis: &DbConn, id: &String) -> bool {
		match redis.zscore::<_,_,String>("subscribers", id) {
			Ok(_value) => true,
			Err(_e) => false
		}
	}

	/*
	 * Checks if id is in friends list
	 */
	 pub fn check_friend(&mut self, redis: &DbConn, id: &String) -> Result<String, SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let subvalue = &[self.id.clone().unwrap(), 
						 ":friends".to_string(),
						].concat();
		match redis.hget::<_,_,String>(subvalue, id) {
			Ok(value) => return Ok(value),
			Err(_e) => { return Err(SubscriberError::new("not found")) }
		}
	 }

	/*
	 * Gets the subscriber id
	 */
	pub fn set_id(&mut self, redis: &DbConn) -> Result<(), SubscriberError> {
		if self.social_id.is_none() {
			return Err(SubscriberError::new("No Social Id"))
		}
		if self.proto.is_none() {
			return Err(SubscriberError::new("No proto"))
		}
		let social_id = &self.social_id.clone().unwrap();
		let proto = &self.proto.clone().unwrap();
				
		// Let's check if our subscriber exists
		let check = match Subscriber::get_instance_from_social_id(&redis, &proto, &social_id) {
			Some(y) => Some(y),
			None => None
		};
		
		// If it doesn't exists return error
		if check.is_none() {
			return Err(SubscriberError::new("Subscriber doesn't exists"))
		} else {
			let temp = &check.unwrap();
			self.id = temp.id.clone();
		}
		return Ok(());
	}
	
	/* 
	 * Adds a friend to subscriber's friend list
	 */
	pub fn set_friend(&mut self, redis: &DbConn, friend: Friend) -> Result<(), SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let subvalue = &[self.id.clone().unwrap(), 
						 ":friends".to_string(),
						].concat();
		let check = self.check_subscriber(&redis, &friend.id);
		if check==true {
			match redis.hset_nx::<_,_,_,bool>(subvalue, &friend.id, &friend.name) {
				Ok(_value) => return Ok(()),
				Err(_e) => { return Err(SubscriberError::new("unknown error")) }
			}
		} else {
			return Err(SubscriberError::new("Subscriber doesn't exist"))
		}
	}

	/* 
	 * Delete a friend to subscriber's friend list
	 */
	pub fn delete_friend(&mut self, redis: &DbConn, id: String) -> Result<(), SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let subvalue = &[self.id.clone().unwrap(), 
						 ":friends".to_string(),
						].concat();
		match redis.hdel::<_,_,bool>(subvalue, &id) {
			Ok(_value) => return Ok(()),
			Err(_e) => { return Err(SubscriberError::new("unknown error")) }
		}
	}

	/* 
	 * Adds a friend to subscriber's friend list
	 */
	pub fn update_friend(&mut self, redis: &DbConn, friend: Friend) -> Result<(), SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let subvalue = &[self.id.clone().unwrap(), 
						 ":friends".to_string(),
						].concat();
		let query = &[friend.id.clone(), 
						 ":profile".to_string(),
						].concat();
		match self.check_friend(&redis, &friend.id) {
			Ok(_) => {
				match redis.hget::<_,_,String>(query, "name") {
					Ok(val) => {
						let mut name = &friend.id;
						if val != "" {
							name = &val;
						}
						match redis.hset::<_,_,_,bool>(subvalue, &friend.id, name) {
								Ok(_value) => return Ok(()),
								Err(_e) => { return Err(SubscriberError::new("unknown error")) }
						}
					},
					Err(_) => {
						return Err(SubscriberError::new("No profile"))
					}
				}
			},
			Err(_) => {
				return Err(SubscriberError::new("Subscriber doesn't exist"))
			}
		}
	}

	/* 
	 * Gets subscriber's friend list
	 */
	pub fn get_friends(&mut self, redis: &DbConn) -> Result <Vec<Friend>, SubscriberError>{
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let query = &[self.id.clone().unwrap(), 
						 ":friends".to_string(),
						].concat();
		match redis.hgetall::<
								&String,
								Vec<(String, String)>
							>(query) {
				Ok(value) => {
					let mut friends: Vec<Friend> = Vec::new();
					for val in value {
						let obj = Friend {
											id: val.0.clone(),
											name:val.1.clone(),
										};
						friends.push(obj);
					}
					return Ok(friends)

				},
				Err(_e) => Err(SubscriberError::new("Friends not found"))
		}
	}

	/* 
	 * Sets the subscriber profile
	 */
	pub fn set_profile(&mut self, redis: &DbConn, profile: Profile) -> Result<(), SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let subvalue = &[self.id.clone().unwrap(), 
						 ":profile".to_string(),
						].concat();
		match redis.hset::<_,_,_,bool>(subvalue, "name", &profile.name) {
			Ok(_value) => return Ok(()),
			Err(e) => { return Err(SubscriberError::new(&e.to_string())) }
		}
	}

	/* 
	 * Gets subscriber's profile
	 */
	pub fn get_profile(&mut self, redis: &DbConn) -> Result <Profile, SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let query = &[self.id.clone().unwrap(), 
						 ":profile".to_string(),
						].concat();
		match redis.hgetall::<
								&String,
								Vec<(String, String)>
							>(query) {
				Ok(value) => {
					let profile = Profile { 
							name: value[0].1.clone()
						};
					return Ok(profile);
				},
				Err(_e) => Err(SubscriberError::new("Profile not found"))
		}
	}
	
	/*
	 * Get other suscriber profile
	 */
	pub fn get_other_profile(&mut self, redis: &DbConn, id: &String) -> Result <Profile, SubscriberError> {
		if self.id.is_none() {
			return Err(SubscriberError::new("No Id"))
		}
		let query = &[id.to_owned(), 
						 ":profile".to_string(),
						].concat();
		match redis.hgetall::<
								&String,
								Vec<(String, String)>
							>(query) {
				Ok(value) => {
					if !value.is_empty(){
						let profile = Profile { 
							name: value[0].1.clone()
						};
						return Ok(profile);
					} else {
						return Err(SubscriberError::new("Profile not found"))
					}
				},
				Err(_e) => Err(SubscriberError::new("Profile not found"))
		}
	}
	
	/*
	 * Creates a subscriber
	 */
	pub fn create(&mut self, redis: DbConn) -> Result<(), SubscriberError> {
		if self.social_id.is_none() {
			return Err(SubscriberError::new("No Social Id"))
		}
		if self.proto.is_none() {
			return Err(SubscriberError::new("No proto"))
		}
		let social_id = &self.social_id.clone().unwrap();
		let proto = &self.proto.clone().unwrap();

		// Let's check if our subscriber already exists
		let check = match Subscriber::get_instance_from_social_id(&redis, &proto, &social_id) {
			Some(y) => Some(y),
			None => None
		};

		// If it doesn't exist we create an account for our subscriber
		if check.is_none() {
			let id = nanoid!(12);
			let subvalue = &[proto.to_owned(), 
							 ":".to_string(), 
							 social_id.to_owned()
							].concat(); 
			match pipe()
					.hset_nx("tokenmap", subvalue, &id)
					.ignore()
					.zadd("subscribers", &id, 0)
					.ignore() 
					.query::<Vec<String>>(&*redis) {
				Ok(_value) => { self.id = Some(id.to_owned()); },
				Err(_e) => {return Err(SubscriberError::new("unknown error"))}
			};
		} else {
			let temp = &check.unwrap();
			self.id = temp.id.clone();
		}
		return Ok(());
	}

}