use cosmrs::{
    crypto::{secp256k1::SigningKey, PublicKey},
    AccountId,
};
use cosmwasm_std::HumanAddr;

use crate::{
    consts,
    crypto::{self, Key},
};

pub struct Account {
    prvk: bip32::XPrv,
    pubk: PublicKey,
}

impl Account {
    pub fn from_mnemonic(s: &str) -> Option<Account> {
        let mnemonic = bip39::Mnemonic::parse(s).ok()?;
        // empty passphrase
        Some(Account::from_seed(mnemonic.to_seed("")))
    }

    pub fn from_seed(seed: [u8; 64]) -> Account {
        let path = consts::SCRT_DERIVATION_PATH
            .parse()
            .expect("invalid scrt derivation path");
        let prvk =
            bip32::XPrv::derive_from_path(seed, &path).expect("private key derivation failed");
        let pubk = SigningKey::from(&prvk).public_key();
        Account { prvk, pubk }
    }

    pub fn human_address(&self) -> HumanAddr {
        self.id().as_ref().into()
    }

    pub(crate) fn signing_key(&self) -> SigningKey {
        SigningKey::from(&self.prvk)
    }

    pub(crate) fn id(&self) -> AccountId {
        self.pubk
            .account_id(consts::CHAIN_PREFIX)
            .expect("invalid public key type")
    }

    pub(crate) fn prv_pub_bytes(&self) -> (Key, Key) {
        crypto::edd25519_keys(&self.prvk)
    }
}

pub fn a() -> Account {
    Account::from_mnemonic(A_MNEMONIC).unwrap()
}

pub fn b() -> Account {
    Account::from_mnemonic(B_MNEMONIC).unwrap()
}

pub fn c() -> Account {
    Account::from_mnemonic(C_MNEMONIC).unwrap()
}

pub fn d() -> Account {
    Account::from_mnemonic(D_MNEMONIC).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn accounts_from_mnemonic() {
        assert_eq!(
            a().human_address(),
            HumanAddr::from("secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03")
        );
        assert_eq!(
            b().human_address(),
            HumanAddr::from("secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne")
        );
        assert_eq!(
            c().human_address(),
            HumanAddr::from("secret1ajz54hz8azwuy34qwy9fkjnfcrvf0dzswy0lqq")
        );
        assert_eq!(
            d().human_address(),
            HumanAddr::from("secret1ldjxljw7v4vk6zhyduywh04hpj0jdwxsmrlatf")
        );
    }
}

static A_MNEMONIC: &str = "grant rice replace explain federal release fix clever romance raise often wild taxi quarter soccer fiber love must tape steak together observe swap guitar";
static B_MNEMONIC: &str = "jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow";
static C_MNEMONIC: &str = "chair love bleak wonder skirt permit say assist aunt credit roast size obtain minute throw sand usual age smart exact enough room shadow charge";
static D_MNEMONIC: &str = "word twist toast cloth movie predict advance crumble escape whale sail such angry muffin balcony keen move employ cook valve hurt glimpse breeze brick";
