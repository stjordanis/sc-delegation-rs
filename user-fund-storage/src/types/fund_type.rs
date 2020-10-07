use elrond_wasm::elrond_codec::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FundDescription {
    /// Funds that can only be extracted from contract. Will never be used as stake.
    WithdrawOnly,

    /// Inactive stake, waiting in the queue to be activated.
    Waiting{ created: u64 },

    /// Stake is locked in the protocol and rewards are coming in.
    /// Users cannot withdraw stake, but they can exchange their share of the total stake amongst each other.
    Active,

    /// Same as Active, but no rewards are coming in.
    UnStaked{ created: u64 },

    DeferredPayment{ created: u64 },

}

/// Same as fund description, but only the enum with no additional data.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FundType {
    /// Funds that can only be extracted from contract. Will never be used as stake.
    WithdrawOnly,

    /// Inactive stake, waiting in the queue to be activated.
    Waiting,

    /// Stake is locked in the protocol and rewards are coming in.
    /// Users cannot withdraw stake, but they can exchange their share of the total stake amongst each other.
    Active,

    /// Same as Active, but no rewards are coming in.
    UnStaked,

    DeferredPayment,

}

impl FundType {
    pub const ALL_TYPES: &'static [FundType] = &[
        FundType::WithdrawOnly,
        FundType::Waiting,
        FundType::Active,
        FundType::UnStaked,
        FundType::DeferredPayment,
    ];

    pub fn allow_coalesce(&self) -> bool {
        match self {
            FundType::WithdrawOnly |
            FundType::DeferredPayment => true,
            _ => false,
        }
    }

    pub fn is_stake(&self) -> bool {
        match self {
            FundType::Waiting |
            FundType::Active |
            FundType::UnStaked => true,
            _ => false,
        }
    }

    pub fn funds_in_contract(&self) -> bool {
        match self {
            FundType::WithdrawOnly |
            FundType::Waiting |
            FundType::DeferredPayment => true,
            _ => false,
        }
    }
}

pub const DISCR_WITHDRAW_ONLY: u8 = 0;
pub const DISCR_WAITING: u8 = 1;
pub const DISCR_PENDING_ACT: u8 = 2;
pub const DISCR_ACTIVE_FAILED: u8 = 3;
pub const DISCR_ACTIVE: u8 = 4;
pub const DISCR_UNSTAKED: u8 = 5;
pub const DISCR_DEF_PAYMENT: u8 = 6;
pub const NR_DISCRIMINANTS: u8 = 7;

impl FundDescription {
    #[inline]
    pub fn discriminant(&self) -> u8 {
        self.fund_type().discriminant()
    }

    pub fn fund_type(&self) -> FundType {
        match self {
            FundDescription::WithdrawOnly => FundType::WithdrawOnly,
            FundDescription::Waiting{..} => FundType::Waiting,
            FundDescription::Active => FundType::Active,
            FundDescription::UnStaked{..} => FundType::UnStaked,
            FundDescription::DeferredPayment{..} => FundType::DeferredPayment,
        }
    }
}

impl FundType {
    pub fn discriminant(&self) -> u8 {
        match self {
            FundType::WithdrawOnly => DISCR_WITHDRAW_ONLY,
            FundType::Waiting => DISCR_WAITING,
            FundType::Active => DISCR_ACTIVE,
            FundType::UnStaked => DISCR_UNSTAKED,
            FundType::DeferredPayment => DISCR_DEF_PAYMENT,
        }
    }
}

impl Encode for FundDescription {
	fn dep_encode_to<O: Output>(&self, dest: &mut O) -> Result<(), EncodeError> {
        match self {
            FundDescription::WithdrawOnly => { dest.push_byte(DISCR_WITHDRAW_ONLY); },
            FundDescription::Waiting{ created } => {
                dest.push_byte(DISCR_WAITING);
                created.dep_encode_to(dest)?;
            },
            FundDescription::Active => { dest.push_byte(DISCR_ACTIVE); },
            FundDescription::UnStaked{ created } => {
                dest.push_byte(DISCR_UNSTAKED);
                created.dep_encode_to(dest)?;
            },
            FundDescription::DeferredPayment{ created } => {
                dest.push_byte(DISCR_DEF_PAYMENT);
                created.dep_encode_to(dest)?;
            },
        }
        Ok(())
	}
}

impl Decode for FundDescription {
	fn dep_decode<I: Input>(input: &mut I) -> Result<Self, DecodeError> {
        let discriminant = input.read_byte()?;
        match discriminant {
            DISCR_WITHDRAW_ONLY => Ok(FundDescription::WithdrawOnly),
            DISCR_WAITING => Ok(FundDescription::Waiting{
                created: u64::dep_decode(input)?
            }),
            DISCR_ACTIVE => Ok(FundDescription::Active),
            DISCR_UNSTAKED => Ok(FundDescription::UnStaked{
                created: u64::dep_decode(input)?
            }),
            DISCR_DEF_PAYMENT => Ok(FundDescription::DeferredPayment{
                created: u64::dep_decode(input)?
            }),
            _ => Err(DecodeError::InvalidValue),
        }
    }
}

impl Encode for FundType {
	fn dep_encode_to<O: Output>(&self, dest: &mut O) -> Result<(), EncodeError> {
        match self {
            FundType::WithdrawOnly => { dest.push_byte(DISCR_WITHDRAW_ONLY); },
            FundType::Waiting => { dest.push_byte(DISCR_WAITING);},
            FundType::Active => { dest.push_byte(DISCR_ACTIVE); },
            FundType::UnStaked => { dest.push_byte(DISCR_UNSTAKED);},
            FundType::DeferredPayment => { dest.push_byte(DISCR_DEF_PAYMENT);},
        }
        Ok(())
	}
}

impl Decode for FundType {
	fn dep_decode<I: Input>(input: &mut I) -> Result<Self, DecodeError> {
        let discriminant = input.read_byte()?;
        match discriminant {
            DISCR_WITHDRAW_ONLY => Ok(FundType::WithdrawOnly),
            DISCR_WAITING => Ok(FundType::Waiting),
            DISCR_ACTIVE => Ok(FundType::Active),
            DISCR_UNSTAKED => Ok(FundType::UnStaked),
            DISCR_DEF_PAYMENT => Ok(FundType::DeferredPayment),
            _ => Err(DecodeError::InvalidValue),
        }
    }
}
