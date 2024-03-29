// Copyright (c) 2023.
// All rights reserved by Liam Ren
// This code is licensed under the MIT license.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

use std::{env, net::Ipv4Addr, str::FromStr};

use anyhow::{Context, Result};
use ethers::{
    core::types::{Address, Signature, H256},
    utils::keccak256,
};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Request, Response, Server};
use env_logger;

fn recover_address_from_signature(message: &[u8], signature_str: &str) -> Result<Address> {
    // Hash the message with the Ethereum prefix
    let hashed_msg = keccak256(format!(
        "\x19Ethereum Signed Message:\n{}{:?}",
        message.len(),
        message
    ));

    // Convert the signature to its r, s, and v components
    let signature = Signature::from_str(signature_str).context("Failed to parse signature")?;

    // Recover the Ethereum address
    let address = signature
        .recover(H256::from_slice(&hashed_msg))
        .context("Failed to recover address")?;

    Ok(address)
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>> {
    log::debug!("Request: {:?}", req);

    let mut msg = String::new();

    // Verify the signature in the request header
    if let Some(signature_header) = req.headers().get("X-Signature") {
        let signature = String::from_utf8(signature_header.as_ref().to_vec())
            .context("Failed to parse signature header")?;
        log::debug!("Signature: {}", signature);
        let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

        // Recover the Ethereum address from the signature
        match recover_address_from_signature(body_bytes.as_ref(), signature.as_str()) {
            Ok(address) => {
                msg.push_str(&format!("Signed by address: {:?}", address));
            }
            Err(e) => {
                msg.push_str(&format!(
                    "Failed to recover address from signature: {:?}",
                    e
                ));
            }
        };
    } else {
        msg.push_str("Missing X-Signature header");
    }

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/plain")
        .body(msg.into())
        .context("Failed to build response for a request with valid X-Sinagure header")?)
}

const DEFAULT_IP: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 3000;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let ip = env::var("IP")
        .unwrap_or_else(|_| DEFAULT_IP.to_string())
        .parse::<Ipv4Addr>()
        .context("Failed to parse IP")?;
    let port = env::var("PORT")
        .unwrap_or_else(|_| DEFAULT_PORT.to_string())
        .parse::<u16>()
        .context("Failed to parse PORT")?;

    let addr = (ip, port).into();

    let make_svc = make_service_fn(move |_conn| async move {
        Ok::<_, hyper::Error>(service_fn(move |req| handle_request(req)))
    });

    let server = Server::bind(&addr).serve(make_svc);

    log::info!("Listening on https://{}", addr);

    server.await.context("Failed to start server")?;

    Ok(())
}
