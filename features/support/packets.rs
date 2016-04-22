use gaffer_udp::packet::{
  CompleteGafferPacket,
  GafferPayload
};

use cucumber::InvokeResponse;

use std::str::FromStr;

pub trait FromTable: Sized {
  fn from_table(table: Vec<Vec<String>>) -> Result<Self, InvokeResponse>;
}

impl FromTable for CompleteGafferPacket {
  fn from_table(table: Vec<Vec<String>>) -> Result<CompleteGafferPacket, InvokeResponse> {
    if table.len() != 4 {
      return Err(InvokeResponse::fail_from_str("CompleteGafferPacket expects table to be four rows, [seq, ack_num, ack_field, and payload]"));
    }

    if table.get(0).unwrap().len() != 2 {
      return Err(InvokeResponse::fail_from_str("CompleteGafferPacket expects table to be two columns: field_name and value"));
    }

    let mut seq = None;
    let mut ack_seq = None;
    let mut ack_field = None;
    let mut payload = None;

    for row in table.into_iter() {
      let key = row.get(0).unwrap();
      let value = row.get(1).unwrap();

      match key.as_ref() {
        "seq" => {
          seq = Some(try!(u16::from_str(value).map_err(|_| {InvokeResponse::fail_from_str("Could not convert seq to u16")})));
        },
        "ack_seq" => {
          ack_seq = Some(try!(u16::from_str(value).map_err(|_| {InvokeResponse::fail_from_str("Could not convert ack_seq to u16")})));
        },
        "ack_field" => {
          ack_field = Some(try!(u32::from_str(value).map_err(|_| {InvokeResponse::fail_from_str("Could not convert ack_field tf u32")})));
        },
        "payload" => {
          let mut building_payload = Vec::new();
          for num in value.split_whitespace() {
            let val = try!(u8::from_str(num).map_err(|_| {InvokeResponse::fail_from_str("Could not convert a value in payload to u8")}));
            building_payload.push(val);
          }
          building_payload.resize(1016, 0);
          payload = Some(building_payload);
        },
        _ => return Err(InvokeResponse::fail_from_str("Unknown field type in CompleteGafferPacket table"))
      }
    }

    if seq.is_none() || ack_seq.is_none() || ack_field.is_none() || payload.is_none() {
      return Err(InvokeResponse::fail_from_str("CompleteGafferPacket did not find all mandatory fields in table: [seq, ack_num, ack_field, and payload]"));
    }

    Ok(CompleteGafferPacket {
      seq: seq.unwrap(),
      ack_seq: ack_seq.unwrap(),
      ack_field: ack_field.unwrap(),
      payload: payload.unwrap()
    })
  }
}

impl FromTable for GafferPayload {
  fn from_table(table: Vec<Vec<String>>) -> Result<GafferPayload, InvokeResponse> {
    if table.len() != 1 {
      return Err(InvokeResponse::fail_from_str("GafferPayload expects table to be a single row"));
    }

    let mut payload = Vec::new();
    for value in table.get(0).unwrap().into_iter() {
      payload.push(try!(u8::from_str(&value).map_err(|_| InvokeResponse::fail_from_str("Could not convert an entry in GafferPayload to u8"))));
    }
    payload.resize(1016, 0);

    Ok(payload)
  }
}

