//! # ERC20
//!
//! An application that implements bridged ERC20 token assets.
//!
//! ## Overview
//!
//! ETH balances are stored in the tightly-coupled [`asset`] runtime module. When an account holder burns
//! some of their balance, a `Transfer` event is emitted. An external relayer will listen for this event
//! and relay it to the other chain.
//!
//! ## Interface
//!
//! This application implements the [`Application`] trait and conforms to its interface.
//!
//! ### Dispatchable Calls
//!
//! - `burn`: Burn an ERC20 token balance.
//!
#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{self as system, ensure_signed};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResult,
};
use sp_std::prelude::*;
use sp_core::{H160, U256};
use codec::Decode;

use artemis_core::{ChannelId, Application, SubmitOutbound, AssetId, MultiAsset};

mod payload;
use payload::{InboundPayload, OutboundPayload};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
pub trait Config: system::Config {
	type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

	type Assets: MultiAsset<<Self as system::Config>::AccountId>;

	type SubmitOutbound: SubmitOutbound;
}

decl_storage! {
	trait Store for Module<T: Config> as Erc20Module {
		/// Address of the peer application on the Ethereum side.
		Address get(fn address) config(): H160;
	}
}

decl_event! {
    /// Events for the ERC20 module.
	pub enum Event<T>
	where
		AccountId = <T as system::Config>::AccountId,
	{
		Burned(H160, AccountId, H160, U256),
		Minted(H160, H160, AccountId, U256),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// The submitted payload could not be decoded.
		InvalidPayload,
	}
}

decl_module! {

	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

		/// Burn an ERC20 token balance
		#[weight = 0]
		pub fn burn(origin, channel_id: ChannelId, token: H160, recipient: H160, amount: U256) -> DispatchResult {
			let who = ensure_signed(origin)?;

			T::Assets::withdraw(AssetId::Token(token), &who, amount)?;

			let message = OutboundPayload {
				token: token,
				sender: who.clone(),
				recipient: recipient.clone(),
				amount: amount
			};

			T::SubmitOutbound::submit(channel_id, Address::get(), &message.encode())?;

			Self::deposit_event(RawEvent::Burned(token, who.clone(), recipient, amount));

			Ok(())
		}

	}
}

impl<T: Config> Module<T> {
	fn handle_payload(payload: &InboundPayload<T::AccountId>) -> DispatchResult {
		T::Assets::deposit(
			AssetId::Token(payload.token),
			&payload.recipient,
			payload.amount
		)?;
		Self::deposit_event(
			RawEvent::Minted(
				payload.token,
				payload.sender,
				payload.recipient.clone(),
				payload.amount
		));
		Ok(())
	}
}

impl<T: Config> Application for Module<T> {
	// Handle a message submitted to us by an inbound channel.
	fn handle(mut payload: &[u8]) -> DispatchResult {
		let payload_decoded: InboundPayload<T::AccountId> = InboundPayload::decode(&mut payload)
			.map_err(|_| Error::<T>::InvalidPayload)?;

		Self::handle_payload(&payload_decoded)
	}

	fn address() -> H160 {
		Address::get()
	}
}
