#[cfg(test)]
mod tests {
    use colored::Colorize;
    use dlc_messages::{
        message_handler::MessageHandler, AcceptDlc, Message, OfferDlc, SignDlc, WireMessage,
        ACCEPT_TYPE, OFFER_TYPE, SIGN_TYPE,
    };
    use lightning::ln::wire::CustomMessageReader;
    use lightning::util::ser::{Readable, Writeable};
    use std::{char::from_u32, fs, io::Cursor};

    #[test]
    fn dlc_specs_test_vectors() {
        // Optimistically set global test success to true
        let mut success = true;
        // List test vectors paths
        let test_vectors_paths = fs::read_dir("./test_vectors/").unwrap();
        // Loop through all test vectors
        for path in test_vectors_paths {
            if let Some(test_vector_path) = path.unwrap().path().to_str() {
                println!(
                    "\n{} {}\n",
                    "Test vector: ".bold().yellow(),
                    test_vector_path.bold()
                );

                // Test vector from node-dlc
                let test_vector_str = fs::read_to_string(test_vector_path.to_string()).unwrap();
                // println!("{}", test_vector_str);

                // Parse test_vector_str string into serde_json::Value
                let test_vector: serde_json::Value =
                    serde_json::from_str(&test_vector_str).unwrap();

                for msg_type in ["offer_message", "accept_message", "sign_message"] {
                    // println!("{}", msg_type);

                    // Grab message string
                    let msg_str = serde_json::to_string(&test_vector[msg_type]["message"]).unwrap();
                    // println!("{}", msg_str);

                    // Parse msg_str string into message object
                    let msg: Message = match msg_type {
                        "offer_message" => {
                            let offer_msg_res: Result<OfferDlc, serde_json::Error> =
                                serde_json::from_str(&msg_str);
                            let offer_msg = match offer_msg_res {
                                Ok(msg) => msg,
                                Err(_) => panic!("ERROR: could not parse offer_message string"),
                            };
                            Message::Offer(offer_msg)
                        }
                        "accept_message" => {
                            let accept_msg_res: Result<AcceptDlc, serde_json::Error> =
                                serde_json::from_str(&msg_str);
                            let accept_msg = match accept_msg_res {
                                Ok(msg) => msg,
                                Err(_) => panic!("ERROR: could not parse accept_message string"),
                            };
                            Message::Accept(accept_msg)
                        }
                        "sign_message" => {
                            let sign_msg_res: Result<SignDlc, serde_json::Error> =
                                serde_json::from_str(&msg_str);
                            let sign_msg = match sign_msg_res {
                                Ok(msg) => msg,
                                Err(_) => panic!("ERROR: could not parse sign_message string"),
                            };
                            Message::Sign(sign_msg)
                        }
                        _ => panic!("ERROR: unknown msg_type"),
                    };
                    // println!("{:#?}", msg);

                    //Grab serialized message string
                    let serialized: String =
                        serde_json::to_string(&test_vector[msg_type]["serialized"]).unwrap();
                    // Remove quotes
                    let serialized_msg: &str = &serialized[1..serialized.len() - 1];
                    // println!("{}", serialized_msg);

                    // Test serialization
                    let successful_serialization: bool =
                        test_serialization(msg.clone(), serialized_msg, msg_type);
                    // Print serialization test result
                    if successful_serialization {
                        println!(
                            " {} {} serialized successfully\n",
                            from_u32(0x2705).unwrap(),
                            msg_type.bold()
                        );
                    } else {
                        println!(
                            " {} {} serialization failed\n",
                            from_u32(0x274C).unwrap(),
                            msg_type.bold()
                        );
                        success = false;
                    }

                    // Test deserialization
                    let successful_deserialization: bool =
                        test_deserialization(msg, serialized_msg);
                    // Print deserialization test result
                    if successful_deserialization {
                        println!(
                            " {} {} deserialized successfully\n",
                            from_u32(0x2705).unwrap(),
                            msg_type.bold()
                        );
                    } else {
                        println!(
                            " {} {} deserialization failed\n",
                            from_u32(0x274C).unwrap(),
                            msg_type.bold()
                        );
                        success = false;
                    }
                }
            }
        }
        // Assert global test success
        assert!(success);
    }

    /// This function serializes the dlc message of a test vector and returns `true` if the result
    /// equals the `serialized` field of that test vector, `false` otherwise
    fn test_serialization(msg: Message, serialized_msg: &str, msg_type: &str) -> bool {
        // Convert original serialized message hex value into bytes
        let original_msg_bytes = hex::decode(serialized_msg).unwrap();
        // println!("{}", serialized_msg);

        // Serialize the message
        let mut serialized_msg_bytes = Vec::new();
        // Write the message type first
        match msg_type {
            "offer_message" => {
                OFFER_TYPE
                    .write(&mut serialized_msg_bytes)
                    .expect("Error writing offer_message type");
            }
            "accept_message" => {
                ACCEPT_TYPE
                    .write(&mut serialized_msg_bytes)
                    .expect("Error writing accept_message type");
            }
            "sign_message" => {
                SIGN_TYPE
                    .write(&mut serialized_msg_bytes)
                    .expect("Error writing sign_message type");
            }
            _ => panic!("ERROR: unknown msg_type"),
        }
        // Then write the message itself
        msg.write(&mut serialized_msg_bytes)
            .expect("Error writing message");
        // let serialized_msg_hex = hex::encode(&serialized_msg_bytes);
        // println!("{}", serialized_msg_hex);

        let successful_serialization: bool = (&original_msg_bytes).eq(&serialized_msg_bytes);
        successful_serialization
    }

    /// This function deserializes the `serialized` field of a test vector and returns `true` if the
    /// result equals the original dlc message of that test vector, `false` otherwise
    fn test_deserialization(msg: Message, serialized_msg: &str) -> bool {
        // Convert serialized message hex value into bytes
        let mut msg_bytes = hex::decode(serialized_msg).unwrap();

        // Instantiate a new reader
        let mut msg_reader = Cursor::new(&mut msg_bytes);

        // The Readable trait is already implemented for u16 (see: https://github.com/lightningdevkit/rust-lightning/blob/df237ba3b455f0ef246604125b8933a7f0074fc5/lightning/src/util/ser.rs#L516C35-L516C35 and https://github.com/lightningdevkit/rust-lightning/blob/df237ba3b455f0ef246604125b8933a7f0074fc5/lightning/src/util/ser.rs#L473)
        let msg_type_prefix =
            <u16 as Readable>::read(&mut msg_reader).expect("to be able to read the type prefix.");
        // println!("Message type prefix: {}", msg_type_prefix);

        // Instantiate new MessageHandler
        let msg_handler = MessageHandler::new();
        // Decode serialized message
        let decoded_wire_msg: WireMessage =
            MessageHandler::read(&msg_handler, msg_type_prefix, &mut msg_reader)
                .expect("to be able to read the message")
                .unwrap();
        // println!("Decoded message: {:#?}", &decoded_msg);

        // Grab the decoded message and check that decoded message equals original message
        let successful_deserialization: bool = match decoded_wire_msg {
            WireMessage::Message(Message::Offer(decoded_offer_msg)) => {
                if let WireMessage::Message(Message::Offer(original_offer_msg)) =
                    WireMessage::Message(msg)
                {
                    PartialEq::eq(&original_offer_msg, &decoded_offer_msg)
                } else {
                    panic!("ERROR: could not assign to original_offer_msg")
                }
            }
            WireMessage::Message(Message::Accept(decoded_accept_msg)) => {
                if let WireMessage::Message(Message::Accept(original_accept_msg)) =
                    WireMessage::Message(msg)
                {
                    PartialEq::eq(&original_accept_msg, &decoded_accept_msg)
                } else {
                    panic!("ERROR: could not assign to original_accept_msg")
                }
            }
            WireMessage::Message(Message::Sign(decoded_sign_msg)) => {
                if let WireMessage::Message(Message::Sign(original_sign_msg)) =
                    WireMessage::Message(msg)
                {
                    PartialEq::eq(&original_sign_msg, &decoded_sign_msg)
                } else {
                    panic!("ERROR: could not assign to original_sign_msg")
                }
            }
            _ => panic!("ERROR: wrong Type for decoded_msg"),
        };
        successful_deserialization
    }
}
