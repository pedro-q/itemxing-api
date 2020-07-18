use crate::DbConn;  
use rocket_contrib::databases::redis::{ Commands};
use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct Item {
	pub class: String,
	pub id: String,
	pub donated: bool,
	pub stock: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Class(pub String);

#[derive(Debug)]
pub struct ItemError {
	pub details: String
}

impl ItemError {
	fn new(msg: &str) -> ItemError {
		ItemError{details: msg.to_string()}
	}
}

impl fmt::Display for ItemError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{}",self.details)
	}
}

impl error::Error for ItemError {
	fn description(&self) -> &str {
		&self.details
	}
}

impl Class {
	pub fn get_class(
						&mut self, 
						redis:DbConn, 
						subscriber_id: &String, 
					) -> Result<Vec<Item>, ItemError> {
		let query = &[
						subscriber_id.to_owned(),
						":".to_string(), 
						self.0.to_owned()
					].concat();
		match redis.hgetall::<
								&String,
								Vec<(String, String)>
							>(query) {
				Ok(value) => {
					let mut items: Vec<Item> = Vec::new();
					for val in value {
						let meta:Vec<&str> = val.1.split(":").collect();
						let donated: bool = meta[0].parse().unwrap();
						let stock: u32 = meta[1].parse().unwrap();
						let obj = Item {
											class: self.0.to_owned(),
											id:val.0.clone(),
											donated: donated,
											stock: stock,
										};
						items.push(obj);
					}
					return Ok(items)

				},
				Err(_e) => Err(ItemError::new("Items not found"))
		}
	}

	pub fn set_class(
					 &mut self,
					 redis:DbConn,
					 subscriber_id: &String,
					 items: Vec<Item>
					) -> Result<(), ItemError>{

		/* I think this would fail anyways but let's 
		 * avoid the user overwriting their friends list
		 * or profile
		*/
		if self.0 == "profile" || self.0 == "friends" {
			return Err(ItemError::new("Invalid class"))
		}
		let query = &[
						subscriber_id.to_owned(),
						":".to_string(), 
						self.0.to_owned()
					].concat();
		// We need to process the Items into something simple for redis
		let mut items_pr = Vec::new();
		for item in items {
			let meta = [item.donated.to_string(), 
						 ":".to_string(), 
						 item.stock.to_string()
						 ].concat();
			let obj = (item.id.to_owned(), meta);
			items_pr.push(obj);
		}
		if !items_pr.is_empty() {
			match redis.hset_multiple::<_, _, _, String>(query, &items_pr){
				Ok(_) => {
					return Ok(());
				},
				Err(_e) => Err(ItemError::new("Adding items failed"))
			
			}
		} else {
			return Ok(());
		}
	}

}
