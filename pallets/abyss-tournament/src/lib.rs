// Copyright 2021-2023 UINB Technologies Pte. Ltd.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

#[frame_support::pallet]
pub mod pallet {
    use ascii::AsciiStr;
    use chrono::NaiveDateTime;
    use frame_support::traits::fungibles::Mutate;
    use frame_support::traits::{tokens::BalanceConversion, Time};
    use frame_support::{pallet_prelude::*, transactional};
    use frame_system::pallet_prelude::*;
    use fuso_support::chainbridge::*;
    use fuso_support::traits::{ReservableToken, Token};
    use pallet_chainbridge as bridge;
    use sp_core::bounded::BoundedBTreeMap;
    use sp_runtime::traits::{TrailingZeroInput, Zero};
    use sp_std::vec::Vec;

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct NPC {
        pub name: Vec<u8>,
        pub img_url: Vec<u8>,
        pub story: Vec<u8>,
        pub features: Vec<u8>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct VoteSelect {
        pub battle_id: BattleId,
        pub npc_id: NpcId,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct VoteSelectInfo {
        pub ticket_amount: u32,
        pub selects: Vec<VoteSelect>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct Battle {
        pub season: SeasonId,
        pub season_name: Vec<u8>,
        pub home: NpcId,
        pub battle_type: BattleType,
        pub visiting: NpcId,
        pub status: BattleStatus,
        pub start_time: u64,
        pub position: u8,
        pub home_score: Option<u8>,
        pub visiting_score: Option<u8>,
        pub video_url: Vec<u8>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct BattleAbstract {
        pub season: SeasonId,
        pub home: NpcId,
        pub visiting: NpcId,
        pub start_time: u64,
        pub home_score: u8,
        pub visiting_score: u8,
    }

    impl Into<BattleAbstract> for Battle {
        fn into(self) -> BattleAbstract {
            BattleAbstract {
                season: self.season,
                home: self.home,
                visiting: self.visiting,
                start_time: self.start_time,
                home_score: self.home_score.unwrap(),
                visiting_score: self.visiting_score.unwrap(),
            }
        }
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct Odds {
        pub battle: Vec<(BattleId, NpcId)>,
        pub o: u128,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct Betting {
        pub battles: Vec<BattleId>,
        pub odds: Vec<Odds>,
        pub season: SeasonId,
        pub start_time: u64,
    }
    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
    pub struct Season<AccountId, Balance> {
        pub id: SeasonId,
        pub name: Vec<u8>,
        pub status: SeasonStatus,
        pub treasury: AccountId,
        pub start_time: u64,
        pub total_battles: u8,
        pub bonus_strategy: Vec<(u8, u32, Balance)>,
        pub ticket_price: Balance,
        pub first_round_battle_type: BattleType,
        pub current_round_battle_type: BattleType,
        pub champion: Option<NpcId>,
        pub total_tickets: u32,
    }

    pub const PALLET_ID: frame_support::PalletId = frame_support::PalletId(*b"abytourn");

    pub type TokenId<T> =
        <<T as Config>::Assets as Token<<T as frame_system::Config>::AccountId>>::TokenId;

    type AssetId<T> = <<T as bridge::Config>::Fungibles as Token<
        <T as frame_system::Config>::AccountId,
    >>::TokenId;

    type BalanceOf<T> = <<T as bridge::Config>::Fungibles as Token<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    type ObjectId = u32;

    type SeasonId = ObjectId;

    type NpcId = ObjectId;

    type BattleId = ObjectId;

    type SelectIndex = u16;

    type BattleAmount = u8;

    type Score = u8;

    #[pallet::config]
    pub trait Config: frame_system::Config + bridge::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type Assets: ReservableToken<Self::AccountId>;

        type BalanceConversion: BalanceConversion<BalanceOf<Self>, AssetId<Self>, BalanceOf<Self>>;

        type BridgeOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        type TimeProvider: Time;

        type OrganizerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        #[pallet::constant]
        type AwtTokenId: Get<AssetId<Self>>;

        #[pallet::constant]
        type MaxTicketAmount: Get<u32>;

        #[pallet::constant]
        type DonorAccount: Get<Self::AccountId>;

        #[pallet::constant]
        type DonationForAgent: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type MaxParticipantPerBattle: Get<u32>;

        #[pallet::constant]
        type BvbTreasury: Get<Self::AccountId>;
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, Default)]
    pub enum SeasonStatus {
        #[default]
        Initial,
        Active,
        Finalized,
    }
    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, Default)]
    pub enum BattleStatus {
        #[default]
        Running,
        Completed,
        Finalized,
    }

    #[derive(Encode, Decode, Default, Clone, PartialEq, Eq, TypeInfo, Debug)]
    pub enum BattleType {
        #[default]
        SixteenthFinals,
        EighthFinals,
        QuarterFinals,
        SemiFinals,
        Finals,
        Regular,
    }

    impl Into<u8> for BattleType {
        fn into(self) -> u8 {
            match self {
                BattleType::SixteenthFinals => 16u8,
                BattleType::EighthFinals => 8u8,
                BattleType::QuarterFinals => 4u8,
                BattleType::SemiFinals => 2u8,
                BattleType::Finals => 1u8,
                BattleType::Regular => 255u8,
            }
        }
    }

    impl TryFrom<u8> for BattleType {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                255 => Ok(BattleType::Regular),
                16 => Ok(BattleType::SixteenthFinals),
                8 => Ok(BattleType::EighthFinals),
                4 => Ok(BattleType::QuarterFinals),
                2 => Ok(BattleType::SemiFinals),
                1 => Ok(BattleType::Finals),
                _ => Err(()),
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        BotCreated(T::AccountId, TokenId<T>, TokenId<T>),
        NowTime(u64),
        NpcPoints(SeasonId, NpcId, BattleAmount, BattleAmount, u32, i32),
        ParticipantPoints(SeasonId, T::AccountId, BattleAmount, BattleAmount, u32),
        ParticipantPointRecord(T::AccountId, BattleId, BattleAbstract, NpcId),
        Battle(BattleId, Battle),
        BattleResult(BattleId, Score, Score, Vec<u8>),
        SeasonUpdate(Season<T::AccountId, BalanceOf<T>>, bool),
    }

    #[pallet::error]
    pub enum Error<T> {
        SeasonNotFound,
        NpcNotFound,
        BattleNotFound,
        NpcNotInBattle,
        BattleTimeError,
        BattleStatusError,
        DuplicateBetting,
        BattleNpcCantSame,
        BettingOverTime,
        TimeFormatError,
        InvalidResourceId,
        SeasonStatusError,
        DefaultSeasonNotFound,
        TicketAmountError,
        TicketPriceTooSmall,
        ParticipantOverflow,
        HaveNoBonus,
        InsufficientBalance,
        BuyTicketOverTime,
        BattleNotInSeason,
        VoteSelectZero,
        SelectIndexOverflow,
        BattleTypeError,
        AddrListInputError,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::type_value]
    pub fn DefaultNextId<T: Config>() -> ObjectId {
        1
    }

    #[pallet::storage]
    #[pallet::getter(fn next_season_id)]
    pub type NextSeasonId<T: Config> = StorageValue<_, ObjectId, ValueQuery, DefaultNextId<T>>;

    #[pallet::storage]
    #[pallet::getter(fn next_npc_id)]
    pub type NextNpcId<T: Config> = StorageValue<_, ObjectId, ValueQuery, DefaultNextId<T>>;

    #[pallet::storage]
    #[pallet::getter(fn next_battle_id)]
    pub type NextBattleId<T: Config> = StorageValue<_, ObjectId, ValueQuery, DefaultNextId<T>>;

    #[pallet::storage]
    #[pallet::getter(fn default_season)]
    pub type DefaultSeason<T: Config> = StorageValue<_, SeasonId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_npc_info)]
    pub type Npcs<T: Config> = StorageMap<_, Twox64Concat, NpcId, NPC, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_battle_info)]
    pub type Battles<T: Config> = StorageMap<_, Twox64Concat, BattleId, Battle, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_season_winners)]
    pub type SeasonWinners<T: Config> = StorageMap<
        _,
        Twox64Concat,
        SeasonId,
        BoundedBTreeMap<BattleId, NpcId, T::MaxParticipantPerBattle>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_season_info)]
    pub type Seasons<T: Config> =
        StorageMap<_, Twox64Concat, SeasonId, Season<T::AccountId, BalanceOf<T>>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_participant_point)]
    pub type ParticipantPoints<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SeasonId,
        Blake2_128Concat,
        (T::AccountId, SelectIndex),
        (BattleAmount, BattleAmount, u32),
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_npc_point)]
    pub type NpcPoints<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SeasonId,
        Blake2_128Concat,
        NpcId,                                  //NpcId, battle_type,
        (BattleAmount, BattleAmount, u32, i32), //total_game, wingame, points
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_ticket)]
    pub type Tickets<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SeasonId,
        Blake2_128Concat,
        T::AccountId,
        (u32, u32), //(total_buy_ticket, remain_ticket_amount)
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_invite_code)]
    pub type InviteCode<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (Vec<u8>, u32, BalanceOf<T>), ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_invite_records)]
    pub type InviteRecords<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_vote_infos)]
    pub type VoteSelectInfos<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        SeasonId,
        (Vec<VoteSelectInfo>, bool),
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_votes_for_cal)]
    pub type VotesForCalc<T: Config> = StorageMap<
        _,
        Twox64Concat,
        BattleId,
        BoundedBTreeMap<(T::AccountId, SelectIndex), NpcId, T::MaxParticipantPerBattle>,
        ValueQuery,
    >;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T::Fungibles as Token<<T as frame_system::Config>::AccountId>>::Balance:
            From<u128> + Into<u128>,
        <T::Fungibles as Token<<T as frame_system::Config>::AccountId>>::TokenId: Into<u32>,
        <T as frame_system::Config>::BlockNumber: Into<u32>,
        <T::TimeProvider as Time>::Moment: Into<u64>,
    {
        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn create_npc(
            origin: OriginFor<T>,
            name: Vec<u8>,
            img_url: Vec<u8>,
            story: Vec<u8>,
            features: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let npc = NPC {
                name,
                img_url,
                story,
                features,
            };
            let id = Self::next_npc_id();
            Npcs::<T>::insert(id, npc);
            NextNpcId::<T>::mutate(|id| *id += 1);
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn create_season(
            origin: OriginFor<T>,
            name: Vec<u8>,
            start_time_str: Vec<u8>,
            total_battles: BattleAmount,
            first_round_battle_type: BattleType,
            ticket_price: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let start_time = Self::date_to_timestamp(start_time_str)
                .map_err(|_e| Error::<T>::TimeFormatError)?;
            ensure!(
                ticket_price > 1000000000000000000.into(),
                Error::<T>::TicketPriceTooSmall
            );
            let id = Self::next_season_id();
            let treasury = Self::get_season_treasury(id);
            let season = Season {
                id,
                name,
                status: SeasonStatus::Initial,
                treasury,
                start_time,
                total_battles,
                bonus_strategy: Vec::default(),
                ticket_price,
                first_round_battle_type: first_round_battle_type.clone(),
                current_round_battle_type: first_round_battle_type,
                champion: None,
                total_tickets: 0u32,
            };
            Seasons::<T>::insert(id, season.clone());
            NextSeasonId::<T>::mutate(|id| *id += 1);
            Self::deposit_event(Event::SeasonUpdate(season, false));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn update_season_current_round(
            origin: OriginFor<T>,
            season_id: SeasonId,
            battle_type: BattleType,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let mut s = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            s.current_round_battle_type = battle_type;
            Seasons::<T>::insert(season_id, s.clone());
            let is_default = Self::default_season() == season_id;
            Self::deposit_event(Event::SeasonUpdate(s, is_default));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn update_season(
            origin: OriginFor<T>,
            season_id: SeasonId,
            name: Vec<u8>,
            start_time_str: Vec<u8>,
            total_battles: BattleAmount,
            first_round_battle_type: BattleType,
            ticket_price: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let start_time = Self::date_to_timestamp(start_time_str)
                .map_err(|_e| Error::<T>::TimeFormatError)?;
            ensure!(
                ticket_price > 1000000000000000000.into(),
                Error::<T>::TicketPriceTooSmall
            );
            let mut s = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            s.name = name;
            s.start_time = start_time;
            s.total_battles = total_battles;
            s.first_round_battle_type = first_round_battle_type;
            s.ticket_price = ticket_price;
            Seasons::<T>::insert(season_id, s.clone());
            let is_default = Self::default_season() == season_id;
            Self::deposit_event(Event::SeasonUpdate(s, is_default));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn create_betting(
            origin: OriginFor<T>,
            _battles: Vec<BattleId>,
            _odds: Vec<Odds>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;

            Ok(().into())
        }

        #[pallet::weight(195_000_000)]
        pub fn set_default_season(
            origin: OriginFor<T>,
            sid: SeasonId,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let s = Self::get_season_info(sid).ok_or(Error::<T>::SeasonNotFound)?;
            let default_season_id = Self::default_season();
            if let Some(default_season) = Self::get_season_info(default_season_id) {
                Self::deposit_event(Event::SeasonUpdate(default_season, false));
            }
            DefaultSeason::<T>::set(sid);
            Self::deposit_event(Event::SeasonUpdate(s, true));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn give_away_tickets(
            origin: OriginFor<T>,
            season_id: SeasonId,
            addrs: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let v: Vec<Vec<u8>> = Decode::decode(&mut TrailingZeroInput::new(addrs.as_slice()))
                .map_err(|_e| Error::<T>::AddrListInputError)?;
            for a in v {
                let addr: T::AccountId = Decode::decode(&mut TrailingZeroInput::new(a.as_ref()))
                    .map_err(|_e| Error::<T>::AddrListInputError)?;
                Tickets::<T>::mutate(season_id, addr, |tickets_amount| {
                    tickets_amount.0 = tickets_amount.0 + 1u32;
                    tickets_amount.1 = tickets_amount.1 + 1u32;
                });
            }
            Ok(().into())
        }

        #[pallet::weight(195_000_0000)]
        pub fn give_away_fee(origin: OriginFor<T>, addrs: Vec<u8>) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let v: Vec<Vec<u8>> = Decode::decode(&mut TrailingZeroInput::new(addrs.as_slice()))
                .map_err(|_e| Error::<T>::AddrListInputError)?;
            for a in v {
                let addr: T::AccountId = Decode::decode(&mut TrailingZeroInput::new(a.as_ref()))
                    .map_err(|_e| Error::<T>::AddrListInputError)?;
                let _ = T::Fungibles::transfer_token(
                    &T::DonorAccount::get(),
                    T::Fungibles::native_token_id(),
                    T::DonationForAgent::get(),
                    &addr,
                );
            }
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn create_battle(
            origin: OriginFor<T>,
            season: ObjectId,
            battle_type: BattleType,
            home: ObjectId,
            visiting: ObjectId,
            start_time_str: Vec<u8>,
            position: u8,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let s = Self::get_season_info(season).ok_or(Error::<T>::SeasonNotFound)?;
            Self::get_npc_info(home).ok_or(Error::<T>::NpcNotFound)?;
            Self::get_npc_info(visiting).ok_or(Error::<T>::NpcNotFound)?;
            let start_time = Self::date_to_timestamp(start_time_str)
                .map_err(|_e| Error::<T>::TimeFormatError)?;
            ensure!(start_time >= s.start_time, Error::<T>::BattleTimeError);
            ensure!(home != visiting, Error::<T>::BattleNpcCantSame);
            let battle = Battle {
                season,
                season_name: s.name,
                battle_type,
                home,
                visiting,
                status: BattleStatus::Running,
                start_time,
                home_score: None,
                visiting_score: None,
                video_url: Vec::new(),
                position,
            };
            let id = Self::next_battle_id();
            Battles::<T>::insert(id, battle.clone());
            NextBattleId::<T>::mutate(|id| *id += 1);
            Self::deposit_event(Event::Battle(id, battle));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn withdraw_season_treasury(
            origin: OriginFor<T>,
            season_id: SeasonId,
            to: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let s = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            let _ = T::Fungibles::transfer_token(&s.treasury, T::AwtTokenId::get(), amount, &to)?;
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn update_battle(
            origin: OriginFor<T>,
            battle_id: BattleId,
            season: ObjectId,
            battle_type: BattleType,
            home: ObjectId,
            visiting: ObjectId,
            start_time_str: Vec<u8>,
            position: u8,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let s = Self::get_season_info(season).ok_or(Error::<T>::SeasonNotFound)?;
            Self::get_npc_info(home).ok_or(Error::<T>::NpcNotFound)?;
            Self::get_npc_info(visiting).ok_or(Error::<T>::NpcNotFound)?;
            let start_time = Self::date_to_timestamp(start_time_str)
                .map_err(|_e| Error::<T>::TimeFormatError)?;
            ensure!(start_time >= s.start_time, Error::<T>::BattleTimeError);
            ensure!(home != visiting, Error::<T>::BattleNpcCantSame);
            let battle = Battle {
                season,
                season_name: s.name,
                battle_type,
                home,
                visiting,
                status: BattleStatus::Running,
                start_time,
                home_score: None,
                visiting_score: None,
                video_url: Vec::new(),
                position,
            };
            Battles::<T>::insert(battle_id, battle.clone());
            Self::deposit_event(Event::Battle(battle_id, battle));
            Ok(().into())
        }

        #[pallet::weight(195_000_0000)]
        pub fn deposit(
            origin: OriginFor<T>,
            to: T::AccountId,
            awt: BalanceOf<T>,
            r_id: ResourceId,
        ) -> DispatchResult {
            let _ = T::BridgeOrigin::ensure_origin(origin)?;
            //mint
            let (chain_id, _, maybe_contract) =
                decode_resource_id(r_id).map_err(|_| Error::<T>::InvalidResourceId)?;
            let token_id = T::AssetIdByName::try_get_asset_id(chain_id, maybe_contract)
                .map_err(|_| Error::<T>::InvalidResourceId)?;
            T::Fungibles::mint_into(token_id, &to, awt)?;
            if frame_system::Pallet::<T>::account_nonce(&to) == Zero::zero() {
                let _ = T::Fungibles::transfer_token(
                    &T::DonorAccount::get(),
                    T::Fungibles::native_token_id(),
                    T::DonationForAgent::get(),
                    &to,
                );
            }
            Ok(())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn buy_ticket(
            origin: OriginFor<T>,
            amount: u32,
            invite_code: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let sid = Self::default_season();
            let mut season = Self::get_season_info(sid).ok_or(Error::<T>::DefaultSeasonNotFound)?;
            let now = T::TimeProvider::now();
            ensure!(
                now.into() / 1000 < season.start_time,
                Error::<T>::BuyTicketOverTime
            );
            let owned_ticket_amount = Self::get_ticket(sid, who.clone());
            let max_ticket_amount = T::MaxTicketAmount::get();
            ensure!(
                owned_ticket_amount.0 + amount <= max_ticket_amount && amount > 0,
                Error::<T>::TicketAmountError
            );
            let m: u128 = season.ticket_price.into() * (amount as u128);
            let balance = T::Fungibles::free_balance(&T::AwtTokenId::get(), &who);
            ensure!(m <= balance.into(), Error::<T>::InsufficientBalance);
            let part_to_bonus: u128 = m * 85 / 100;
            T::Fungibles::transfer_token(
                &who,
                T::AwtTokenId::get(),
                part_to_bonus.into(),
                &season.treasury,
            )?;
            let mut part_to_invitor: u128 = 0;
            if let Some(invitor) = Self::invite_code_to_addr(invite_code) {
                if invitor != who {
                    part_to_invitor = m * 5 / 100;
                    T::Fungibles::transfer_token(
                        &who,
                        T::AwtTokenId::get(),
                        part_to_invitor.into(),
                        &invitor,
                    )?;
                    InviteRecords::<T>::insert(who.clone(), true);
                    InviteCode::<T>::mutate(invitor, |v| {
                        v.1 = v.1 + 1;
                        v.2 = v.2 + part_to_invitor.into();
                    });
                }
            }
            let part_to_bvb_treasury: u128 = m - part_to_bonus - part_to_invitor;
            T::Fungibles::transfer_token(
                &who,
                T::AwtTokenId::get(),
                part_to_bvb_treasury.into(),
                &T::BvbTreasury::get(),
            )?;
            Tickets::<T>::mutate(sid, who, |tickets_amount| {
                tickets_amount.0 = tickets_amount.0 + amount as u32;
                tickets_amount.1 = tickets_amount.1 + amount as u32;
            });
            season.total_tickets = season.total_tickets + amount;
            Seasons::<T>::insert(sid, season);

            Ok(().into())
        }

        #[pallet::weight(195_000_000)]
        pub fn close_season(
            origin: OriginFor<T>,
            season_id: SeasonId,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let s = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            let season_total_battles = s.total_battles;
            let mut champion_tickets: u32 = 0;
            let mut runner_up_tickkets: u32 = 0;
            for ((acc, select_index), (_, win_battles, _)) in
                ParticipantPoints::<T>::iter_prefix(season_id)
            {
                let tickets =
                    Self::get_vote_infos(&acc, season_id).0[select_index as usize].ticket_amount;
                if win_battles == season_total_battles {
                    champion_tickets = champion_tickets + (tickets as u32);
                }
                if win_battles + 1 == season_total_battles {
                    runner_up_tickkets = runner_up_tickkets + (tickets as u32);
                }
            }
            let b: u128 = T::Fungibles::free_balance(&T::AwtTokenId::get(), &s.treasury).into();
            let champion_bonus = b * 7 / 8;
            let runnerup_bonus = b - champion_bonus;
            Seasons::<T>::mutate(season_id, |ss| {
                let mut s = ss.take().unwrap();
                let mut v = Vec::new();
                v.push((
                    season_total_battles,
                    champion_tickets,
                    champion_bonus.into(),
                ));
                v.push((
                    season_total_battles - 1,
                    runner_up_tickkets,
                    runnerup_bonus.into(),
                ));
                s.bonus_strategy = v;
                s.status = SeasonStatus::Finalized;
                ss.replace(s);
            });
            let s = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            let is_default = Self::default_season() == season_id;
            Self::deposit_event(Event::SeasonUpdate(s, is_default));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn claim(origin: OriginFor<T>, season_id: SeasonId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let season = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            ensure!(
                season.status == SeasonStatus::Finalized,
                Error::<T>::SeasonStatusError
            );
            let select_infos = Self::get_vote_infos(&who, season_id);
            ensure!(
                select_infos.0.len() > 0 && select_infos.1 == false,
                Error::<T>::HaveNoBonus
            );

            let result_map = Self::get_season_winners(season_id);
            let mut total_claimed: BalanceOf<T> = 0.into();
            for sel in &select_infos.0 {
                let mut select_right_battles: BattleAmount = 0;
                for s in &sel.selects {
                    if s.npc_id == result_map[&s.battle_id] {
                        select_right_battles += 1;
                    }
                }
                for b in &season.bonus_strategy {
                    if select_right_battles == b.0 && b.1 > 0 {
                        let mut bonus_per_ticket = b.2 / b.1.into();
                        bonus_per_ticket = if bonus_per_ticket > season.ticket_price * 100.into() {
                            season.ticket_price * 100.into()
                        } else {
                            bonus_per_ticket
                        };
                        let amount = bonus_per_ticket * sel.ticket_amount.into();
                        total_claimed = total_claimed + amount;
                        break;
                    }
                }
            }
            T::Fungibles::transfer_token(
                &season.treasury,
                T::AwtTokenId::get(),
                total_claimed,
                &who,
            )?;
            VoteSelectInfos::<T>::mutate(&who, season_id, |s| {
                s.1 = true;
            });
            Ok(().into())
        }

        #[pallet::weight(195_000_000)]
        pub fn set_result(
            origin: OriginFor<T>,
            battle_id: ObjectId,
            home_score: Score,
            visiting_score: Score,
            video_url: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let mut battle = Self::get_battle_info(battle_id).ok_or(Error::<T>::BattleNotFound)?;
            let winner = if home_score > visiting_score {
                battle.home
            } else {
                battle.visiting
            };
            let _ = SeasonWinners::<T>::mutate(&battle.season, |mp| -> DispatchResult {
                let _ = mp
                    .try_insert(battle_id, winner)
                    .map_err(|_e| Error::<T>::ParticipantOverflow)?;
                Ok(())
            })?;
            battle.home_score = Some(home_score);
            battle.visiting_score = Some(visiting_score);
            battle.video_url = video_url;
            battle.status = BattleStatus::Completed;
            Battles::<T>::insert(battle_id, battle.clone());
            Self::deposit_event(Event::<T>::Battle(battle_id, battle));
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn initial_vote(
            origin: OriginFor<T>,
            season_id: SeasonId,
            tickets_amount: u32,
            votes: Vec<VoteSelect>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(votes.len() > 0, Error::<T>::VoteSelectZero);
            let season = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            let t = Self::get_ticket(season_id, who.clone());
            ensure!(
                t.1 >= tickets_amount && tickets_amount > 0,
                Error::<T>::TicketAmountError
            );
            let new_select_index = Self::get_vote_infos(&who, season_id).0.len();
            let now = T::TimeProvider::now();
            let mut battle_id_vec = Vec::new();
            for s in &votes {
                let npc_id = s.npc_id;
                let battle_id = s.battle_id;
                let battle = Self::get_battle_info(battle_id).ok_or(Error::<T>::BattleNotFound)?;
                ensure!(
                    battle.home == npc_id || battle.visiting == npc_id,
                    Error::<T>::NpcNotInBattle
                );
                ensure!(
                    battle.status == BattleStatus::Running,
                    Error::<T>::BattleStatusError
                );
                ensure!(
                    battle.battle_type == season.first_round_battle_type,
                    Error::<T>::BattleTypeError,
                );
                ensure!(battle.season == season_id, Error::<T>::BattleNotInSeason);
                ensure!(
                    now.into() / 1000 < battle.start_time,
                    Error::<T>::BettingOverTime
                );
                let _ = VotesForCalc::<T>::mutate(battle_id, |mp| -> DispatchResult {
                    let _ = mp
                        .try_insert((who.clone(), new_select_index as SelectIndex), npc_id)
                        .map_err(|_e| Error::<T>::ParticipantOverflow)?;
                    Ok(())
                })?;
                battle_id_vec.push(battle_id);
            }
            ensure!(
                !Self::duplicate(battle_id_vec),
                Error::<T>::DuplicateBetting
            );
            let sel = VoteSelectInfo {
                ticket_amount: tickets_amount,
                selects: votes,
            };
            VoteSelectInfos::<T>::mutate(&who, season_id, |v| {
                v.0.push(sel);
            });
            Tickets::<T>::mutate(season_id, &who, |ta| ta.1 = t.1 - tickets_amount);
            Ok(().into())
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn append_vote(
            origin: OriginFor<T>,
            season_id: SeasonId,
            select_index: SelectIndex,
            select_battle_type: u8,
            select_ticket_amount: u32,
            mut votes: Vec<VoteSelect>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(votes.len() > 0, Error::<T>::VoteSelectZero);
            let season = Self::get_season_info(season_id).ok_or(Error::<T>::SeasonNotFound)?;
            ensure!(
                select_battle_type < <BattleType as Into<u8>>::into(season.first_round_battle_type)
                    && BattleType::try_from(select_battle_type).is_ok(),
                Error::<T>::BattleTypeError
            );
            let now = T::TimeProvider::now();
            let old_select = Self::get_vote_infos(&who, season_id);
            ensure!(
                old_select.0.len() > select_index as usize,
                Error::<T>::SelectIndexOverflow,
            );
            let mut old_select_info: VoteSelectInfo = old_select.0[select_index as usize].clone();
            ensure!(
                select_ticket_amount <= old_select_info.ticket_amount && select_ticket_amount > 0,
                Error::<T>::TicketAmountError
            );
            ensure!(
                !Self::append_vote_check_duplicate(&old_select_info.selects, &votes),
                Error::<T>::DuplicateBetting
            );
            if select_ticket_amount == old_select_info.ticket_amount {
                for s in &votes {
                    let npc_id = s.npc_id;
                    let battle_id = s.battle_id;
                    let battle =
                        Self::get_battle_info(battle_id).ok_or(Error::<T>::BattleNotFound)?;
                    ensure!(
                        battle.status == BattleStatus::Running,
                        Error::<T>::BattleStatusError
                    );
                    ensure!(
                        battle.home == npc_id || battle.visiting == npc_id,
                        Error::<T>::NpcNotInBattle
                    );
                    ensure!(battle.season == season_id, Error::<T>::BattleNotInSeason);
                    ensure!(
                        now.into() / 1000 < battle.start_time,
                        Error::<T>::BettingOverTime
                    );
                    old_select_info.selects.push(s.clone());
                    let _ = VotesForCalc::<T>::mutate(battle_id, |mp| -> DispatchResult {
                        let _ = mp
                            .try_insert((who.clone(), select_index as SelectIndex), npc_id)
                            .map_err(|_e| Error::<T>::ParticipantOverflow)?;
                        Ok(())
                    })?;
                }
                VoteSelectInfos::<T>::mutate(&who, season_id, |value| {
                    value.0[select_index as usize] = old_select_info;
                });
                Ok(().into())
            } else {
                VoteSelectInfos::<T>::mutate(&who, season_id, |value| {
                    value.0[select_index as usize].ticket_amount =
                        value.0[select_index as usize].ticket_amount - select_ticket_amount;
                });
                let mut base_select = old_select_info.selects.clone();
                let new_select_index = Self::get_vote_infos(&who, season_id).0.len();
                for s in &votes {
                    let npc_id = s.npc_id;
                    let battle_id = s.battle_id;
                    let battle =
                        Self::get_battle_info(battle_id).ok_or(Error::<T>::BattleNotFound)?;
                    ensure!(
                        battle.status == BattleStatus::Running,
                        Error::<T>::BattleStatusError
                    );
                    ensure!(
                        battle.home == npc_id || battle.visiting == npc_id,
                        Error::<T>::NpcNotInBattle
                    );
                    ensure!(battle.season == season_id, Error::<T>::BattleNotInSeason);
                    ensure!(
                        now.into() / 1000 < battle.start_time,
                        Error::<T>::BettingOverTime
                    );
                }

                base_select.append(&mut votes);
                for sel in &base_select {
                    let _ = VotesForCalc::<T>::mutate(sel.battle_id, |mp| -> DispatchResult {
                        let _ = mp
                            .try_insert((who.clone(), new_select_index as SelectIndex), sel.npc_id)
                            .map_err(|_e| Error::<T>::ParticipantOverflow)?;
                        Ok(())
                    })?;
                }

                let sel = VoteSelectInfo {
                    ticket_amount: select_ticket_amount,
                    selects: base_select,
                };
                VoteSelectInfos::<T>::mutate(&who, season_id, |v| {
                    v.0.push(sel);
                });
                let points = Self::get_participant_point(&season_id, (&who, select_index));
                ParticipantPoints::<T>::insert(
                    season_id,
                    (who, new_select_index as SelectIndex),
                    points,
                );

                Ok(().into())
            }
        }

        #[transactional]
        #[pallet::weight(195_000_000)]
        pub fn settle(origin: OriginFor<T>, battle_id: BattleId) -> DispatchResultWithPostInfo {
            let _ = T::OrganizerOrigin::ensure_origin(origin)?;
            let battle: Battle =
                Self::get_battle_info(battle_id).ok_or(Error::<T>::BattleNotFound)?;
            let battle_season = battle.season;
            let battle_type = battle.battle_type.clone();
            ensure!(
                battle.status == BattleStatus::Completed,
                Error::<T>::BattleStatusError
            );
            let mut score_diff: Score = 0;
            let mut winner: NpcId = 0;
            let mut loser: NpcId = 0;
            match battle.home_score > battle.visiting_score {
                true => {
                    score_diff = battle.home_score.unwrap() - battle.visiting_score.unwrap();
                    winner = battle.home;
                    loser = battle.visiting;
                }
                false => {
                    score_diff = battle.visiting_score.unwrap() - battle.home_score.unwrap();
                    winner = battle.visiting;
                    loser = battle.home;
                }
            };
            Self::update_npc_point(
                battle.season,
                winner,
                loser,
                score_diff,
                battle.battle_type.clone(),
            )?;
            Self::update_participant_point(battle.season, battle_id, battle, winner)?;
            Battles::<T>::mutate(battle_id, |b| {
                let mut battle = b.take().unwrap();
                battle.status = BattleStatus::Finalized;
                b.replace(battle);
            });

            if battle_type == BattleType::Finals {
                Seasons::<T>::mutate(battle_season, |ss| {
                    let mut s = ss.take().unwrap();
                    s.champion = Some(winner);
                    ss.replace(s);
                });
                let s = Self::get_season_info(battle_season).ok_or(Error::<T>::SeasonNotFound)?;
                let is_default = Self::default_season() == battle_season;
                Self::deposit_event(Event::SeasonUpdate(s, is_default));
            }

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn append_vote_check_duplicate(
            old_select: &Vec<VoteSelect>,
            new_select: &Vec<VoteSelect>,
        ) -> bool {
            let mut old_battle_ids = Vec::new();
            for s in old_select {
                old_battle_ids.push(s.battle_id);
            }
            for s in new_select {
                if old_battle_ids.contains(&s.battle_id) {
                    return true;
                } else {
                    old_battle_ids.push(s.battle_id);
                }
            }
            false
        }

        pub fn invite_code_to_addr(invite_code: Vec<u8>) -> Option<T::AccountId> {
            if let Ok(mut v) = base64::decode(invite_code) {
                v.reverse();
                let addr: Option<T::AccountId> =
                    Decode::decode(&mut TrailingZeroInput::new(v.as_slice())).ok();
                return addr;
            }
            None
        }

        pub fn addr_to_invite_code(addr: T::AccountId) -> Vec<u8> {
            let mut r = addr.encode();
            r.reverse();
            base64::encode(r).into_bytes()
        }

        pub fn update_participant_point(
            season_id: SeasonId,
            battle_id: BattleId,
            battle: Battle,
            winner: NpcId,
        ) -> DispatchResult {
            let battle_abstract: BattleAbstract = battle.into();
            VotesForCalc::<T>::mutate(battle_id, |v| {
                for ((acc, select_index), npc_id) in v {
                    let nid = *npc_id;
                    if winner == nid {
                        ParticipantPoints::<T>::mutate(&season_id, (&acc, select_index), |p| {
                            p.0 += 1;
                            p.1 += 1;
                            p.2 += 3;
                            Self::deposit_event(Event::ParticipantPoints(
                                season_id,
                                acc.clone(),
                                p.0,
                                p.1,
                                p.2,
                            ));
                        });
                    }
                    if winner != nid {
                        ParticipantPoints::<T>::mutate(&season_id, (&acc, select_index), |p| {
                            p.0 += 1;
                            Self::deposit_event(Event::ParticipantPoints(
                                season_id,
                                acc.clone(),
                                p.0,
                                p.1,
                                p.2,
                            ));
                        });
                    }
                    Self::deposit_event(Event::ParticipantPointRecord(
                        acc.clone(),
                        battle_id,
                        battle_abstract.clone(),
                        nid,
                    ));
                }
            });
            Ok(())
        }

        pub fn update_npc_point(
            season_id: SeasonId,
            winner: NpcId,
            loser: NpcId,
            score_diff: Score,
            battle_type: BattleType,
        ) -> DispatchResult {
            NpcPoints::<T>::mutate(&season_id, &winner, |e| {
                e.0 = e.0 + 1;
                e.1 = e.1 + 1;
                e.2 = e.2 + 3;
                e.3 = e.3 + (score_diff as i32);
                if battle_type == BattleType::Regular {
                    Self::deposit_event(Event::NpcPoints(season_id, winner, e.0, e.1, e.2, e.3));
                }
            });
            NpcPoints::<T>::mutate(&season_id, &loser, |e| {
                e.0 = e.0 + 1;
                e.3 = e.3 - (score_diff as i32);
                if battle_type == BattleType::Regular {
                    Self::deposit_event(Event::NpcPoints(season_id, loser, e.0, e.1, e.2, e.3));
                }
            });
            Ok(())
        }

        fn duplicate(v: Vec<BattleId>) -> bool {
            if v.len() == 1 {
                return false;
            }
            for i in 0..v.len() - 1 {
                for j in i + 1..v.len() {
                    if v[i] == v[j] {
                        return true;
                    }
                }
            }
            false
        }

        pub fn get_season_treasury(season_id: SeasonId) -> T::AccountId {
            let h = (b"-*-#fusotao-abyssworld-season#-*-", season_id)
                .using_encoded(sp_io::hashing::blake2_256);
            Decode::decode(&mut h.as_ref()).expect("32 bytes; qed")
        }

        pub fn date_to_timestamp(v: Vec<u8>) -> Result<u64, u8> {
            let fmt = "%Y-%m-%d %H:%M:%S";
            let dt = AsciiStr::from_ascii(&v).map_err(|_e| 0u8)?;
            let s = NaiveDateTime::parse_from_str(dt.as_str(), fmt).map_err(|_e| 1u8)?;
            let timestamp: u64 = s.timestamp() as u64;
            Ok(timestamp)
        }
    }
}
