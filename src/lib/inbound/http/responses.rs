use serde::Serialize;

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct ResponseBody<T: Serialize> {
  status_code: u16,
  data: T, 
  }

#[derive(Debug,Clone,PartialEq,Serialize)]
pub struct ErrorResponseData {
  pub message: String,
  }
