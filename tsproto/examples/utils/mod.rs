use std::net::SocketAddr;

use anyhow::Result;
use futures::prelude::*;
use slog::{info, o, warn, Drain, Level, Logger};
use tokio::net::UdpSocket;
use tsproto::algorithms as algs;
use tsproto::client::Client;
use tsproto::connection::StreamItem;
use tsproto::crypto::EccKeyPrivP256;
use tsproto_packets::packets::*;

pub fn create_logger() -> Logger {
	let decorator = slog_term::TermDecorator::new().build();
	let drain = slog_term::CompactFormat::new(decorator).build().fuse();
	let drain = slog_async::Async::new(drain).build().fuse();

	slog::Logger::root(drain, o!())
}

pub async fn create_client(
	local_address: SocketAddr,
	remote_address: SocketAddr,
	logger: Logger,
	verbose: u8,
) -> Result<Client>
{
	// Get P-256 ECDH key
	let private_key = EccKeyPrivP256::import_str(
		"MG0DAgeAAgEgAiAIXJBlj1hQbaH0Eq0DuLlCmH8bl+veTAO2+\
		k9EQjEYSgIgNnImcmKo7ls5mExb6skfK2Tw+u54aeDr0OP1ITsC/50CIA8M5nm\
		DBnmDM/gZ//4AAAAAAAAAAAAAAAAAAAAZRzOI").unwrap();

	let udp_socket = UdpSocket::bind(local_address).await?;
	let mut con = Client::new(logger, remote_address, udp_socket, private_key);

	if verbose >= 1 {
		tsproto::log::add_logger(con.logger.clone(), verbose - 1, &mut con)
	}

	Ok(con)
}

/// Returns the `initserver` command.
pub async fn connect(con: &mut Client) -> Result<InCommandBuf> {
	con.connect().await?;

	// Send clientinit
	let private_key = EccKeyPrivP256::import_str(
		"MG0DAgeAAgEgAiAIXJBlj1hQbaH0Eq0DuLlCmH8bl+veTAO2+\
		k9EQjEYSgIgNnImcmKo7ls5mExb6skfK2Tw+u54aeDr0OP1ITsC/50CIA8M5nm\
		DBnmDM/gZ//4AAAAAAAAAAAAAAAAAAAAZRzOI").unwrap();

	// Compute hash cash
	let mut time_reporter = slog_perf::TimeReporter::new_with_level(
		"Compute public key hash cash level",
		con.logger.clone(),
		Level::Info,
	);
	time_reporter.start("Compute public key hash cash level");
	let private_key_as_pub = private_key.to_pub();
	let offset = algs::hash_cash(&private_key_as_pub, 8).unwrap();
	let omega = private_key_as_pub.to_ts().unwrap();
	time_reporter.finish();
	info!(con.logger, "Computed hash cash level";
		"level" => algs::get_hash_cash_level(&omega, offset),
		"offset" => offset);

	// Create clientinit packet
	let offset = offset.to_string();
	let packet = OutCommand::new::<
		_,
		_,
		String,
		String,
		_,
		_,
		std::iter::Empty<_>,
	>(
		Direction::C2S,
		PacketType::Command,
		"clientinit",
		vec![
			("client_nickname", "Bot"),
			("client_version", "3.?.? [Build: 5680278000]"),
			("client_platform", "Linux"),
			("client_input_hardware", "1"),
			("client_output_hardware", "1"),
			("client_default_channel", ""),
			("client_default_channel_password", ""),
			("client_server_password", ""),
			("client_meta_data", ""),
			(
				"client_version_sign",
				"Hjd+N58Gv3ENhoKmGYy2bNRBsNNgm5kpiaQWxOj5HN2DXttG6REjymSwJtpJ8muC2gSwRuZi0R+8Laan5ts5CQ==",
			),
			("client_nickname_phonetic", ""),
			("client_key_offset", &offset),
			("client_default_token", ""),
			("client_badges", "Overwolf=0"),
			(
				"hwid",
				"923f136fb1e22ae6ce95e60255529c00,\
				 d13231b1bc33edfecfb9169cc7a63bcc",
			),
		]
		.into_iter(),
		std::iter::empty(),
	);

	let fut = con.send_packet(packet).await;
	Ok(tokio::try_join!(fut, con.filter_commands(|con, cmd|
		Ok(if cmd.data().data().name == "initserver" {
			Some(cmd)
		} else {
			con.hand_back_buffer(cmd.into_buffer());
			None
		})))?.1)
}

pub async fn wait_disconnect(con: &mut Client) -> Result<()> {
	loop {
		let item = con.next().await;
		match item {
			None => return Ok(()),
			Some(Err(e)) => return Err(e),
			Some(Ok(StreamItem::Error(e))) => {
				warn!(con.logger, "Got connection error"; "error" => %e);
			}
			Some(Ok(StreamItem::S2CInit(packet))) => {
				con.hand_back_buffer(packet.into_buffer());
			}
			Some(Ok(StreamItem::C2SInit(packet))) => {
				con.hand_back_buffer(packet.into_buffer());
			}
			Some(Ok(StreamItem::Audio(packet))) => {
				con.hand_back_buffer(packet.into_buffer());
			}
			Some(Ok(StreamItem::Command(packet))) => {
				con.hand_back_buffer(packet.into_buffer());
			}
		}
	}
}

pub async fn disconnect(con: &mut Client) -> Result<()> {
	let packet =
		OutCommand::new::<_, _, String, String, _, _, std::iter::Empty<_>>(
			Direction::C2S,
			PacketType::Command,
			"clientdisconnect",
			vec![
				// Reason: Disconnect
				("reasonid", "8"),
				("reasonmsg", "Bye"),
			]
			.into_iter(),
			std::iter::empty(),
		);

	let fut = con.send_packet(packet).await;
	tokio::try_join!(fut, wait_disconnect(con))?;
	Ok(())
}
