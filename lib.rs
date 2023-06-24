#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod erc20 {
    use ink::storage::Mapping;
    use ink::prelude::string::String;

    #[ink(storage)]
    pub struct Erc20 {
        name: String,
        symbol: String,
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        InsufficientAllowance,
    }

    type Result<T> = core::result::Result<T, Error>;

    impl Erc20 {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(name: String, symbol: String, total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            balances.insert(Self::env().caller(), &total_supply);

            Self::env().emit_event(Transfer { from: None, to: Some(Self::env().caller()), value: total_supply });

            Self {
                name,
                symbol,
                total_supply,
                balances,
                allowances: Default::default(),
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).unwrap_or_default()
        }

        #[ink(message)]
        pub fn name(&self) -> String {
            self.name.clone()
        }

        #[ink(message)]
        pub fn symbol(&self) -> String {
            self.symbol.clone()
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let from = self.env().caller();
            self._transfer(&from, &to, value)
        }

        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let spender = self.env().caller();
            let allowance = self.allowances.get(&(from, spender)).unwrap_or_default();
            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }
            self.allowances.insert((from, spender), &(allowance - value));
            self._transfer(&from, &to, value)
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), &value);

            self.env().emit_event(Approval { owner, spender, value });
            Ok(())
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowances.get(&(owner, spender)).unwrap_or_default()
        }

        #[ink(message)]
        pub fn increase_allowance(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            let allowance = self.allowances.get(&(owner, spender)).unwrap_or_default();
            self.allowances.insert((owner, spender), &(allowance + value));
            Ok(())
        }

        #[ink(message)]
        pub fn decrease_allowance(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            let allowance = self.allowances.get(&(owner, spender)).unwrap_or_default();
            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }
            self.allowances.insert((owner, spender), &(allowance - value));
            Ok(())
        }

        pub fn _transfer(&mut self, from: &AccountId, to: &AccountId, value: Balance) -> Result<()> {
            let balance_from = self.balance_of(*from);
            let balance_to = self.balance_of(*to);

            if value > balance_from {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(balance_from - value));
            self.balances.insert(to, &(balance_to + value));

            self.env().emit_event(Transfer { from: Some(*from), to: Some(*to), value });

            Ok(())
        }

        pub fn _mint(&mut self, to: &AccountId, value: Balance) -> Result<()> {
            let balance_to = self.balance_of(*to);
            self.balances.insert(to, &(balance_to + value));
            self.total_supply += value;

            self.env().emit_event(Transfer { from: None, to: Some(*to), value });

            Ok(())
        }

        pub fn _burn(&mut self, from: &AccountId, value: Balance) -> Result<()> {
            let balance_from = self.balance_of(*from);
            if value > balance_from {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(balance_from - value));
            self.total_supply -= value;

            self.env().emit_event(Transfer { from: Some(*from), to: None, value });

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        type Event = <Erc20 as ::ink::reflect::ContractEventBase>::Type;

        #[ink::test]
        fn constructor_works() {
            let erc20 = Erc20::new(
                String::from("Ink Test Token"),
                String::from("ITT"),
                1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.total_supply(), 1000);
            assert_eq!(erc20.balance_of(accounts.alice), 1000);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emitted_events[0];
            let decoded = <Event as scale::Decode>::decode(&mut &event.data[..]).expect("decoding failed");
            match decoded {
                Event::Transfer(Transfer { from, to, value }) => {
                    assert!(from.is_none(), "mint from error");
                    assert_eq!(to, Some(accounts.alice), "mint to error");
                    assert_eq!(value, 1000, "mint value error");
                }
                _ => panic!("match error event"),
            }
        }

        #[ink::test]
        pub fn transfer_works() {
            let mut erc20 = Erc20::new(
                String::from("Ink Test Token"),
                String::from("ITT"),
                1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.balance_of(accounts.alice), 1000);
            assert_eq!(erc20.balance_of(accounts.bob), 0);

            assert_eq!(erc20.transfer(accounts.bob, 100), Ok(()));
            assert_eq!(erc20.balance_of(accounts.alice), 900);
            assert_eq!(erc20.balance_of(accounts.bob), 100);

            assert_eq!(erc20.transfer(accounts.bob, 1000), Err(Error::InsufficientBalance));
            assert_eq!(erc20.balance_of(accounts.alice), 900);
            assert_eq!(erc20.balance_of(accounts.bob), 100);
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            let mut erc20 = Erc20::new(
                String::from("Ink Test Token"),
                String::from("ITT"),
                1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let res = erc20.transfer(accounts.charlie, 20);
            assert!(res.is_err());
            assert_eq!(res.unwrap_err(), Error::InsufficientBalance);
        }

        #[ink::test]
        fn approve_works() {
            let mut erc20 = Erc20::new(
                String::from("Ink Test Token"),
                String::from("ITT"),
                1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.allowance(accounts.alice, accounts.bob), 0);
            assert_eq!(erc20.approve(accounts.bob, 100), Ok(()));
            assert_eq!(erc20.allowance(accounts.alice, accounts.bob), 100);
        }

        #[ink::test]
        fn transfer_from_works() {
            let mut erc20 = Erc20::new(
                String::from("Ink Test Token"),
                String::from("ITT"),
                1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.balance_of(accounts.alice), 1000);
            assert_eq!(erc20.balance_of(accounts.bob), 0);
            assert_eq!(erc20.allowance(accounts.alice, accounts.bob), 0);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(erc20.transfer_from(accounts.alice, accounts.charlie, 100), Err(Error::InsufficientAllowance));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(erc20.approve(accounts.bob, 100), Ok(()));
            assert_eq!(erc20.allowance(accounts.alice, accounts.bob), 100);
            assert_eq!(erc20.balance_of(accounts.alice), 1000);
            assert_eq!(erc20.balance_of(accounts.charlie), 0);
            assert_eq!(erc20.allowance(accounts.alice, accounts.bob), 100);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(erc20.transfer_from(accounts.alice, accounts.charlie, 100), Ok(()));
            assert_eq!(erc20.balance_of(accounts.alice), 900);
            assert_eq!(erc20.balance_of(accounts.charlie), 100);
            assert_eq!(erc20.allowance(accounts.alice, accounts.bob), 0);
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::build_message;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn transfer_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_supply = 1000;
            let constructor = Erc20Ref::new(total_supply);
            let contract_acc_id = client.instantiate(
                "erc20",
                &ink_e2e::alice(),
                constructor,
                0,
                None,
            ).await.expect("instantiate failed").account_id;

            let alice_acc_id = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc_id = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let transfer_msg = build_message::<Erc20Ref>(
                contract_acc_id.clone()
            ).call(|erc20| erc20.transfer(bob_acc_id.clone(), 100));

            let res = client.call(
                &ink_e2e::alice(),
                transfer_msg,
                0,
                None).await;

            assert!(res.is_ok());

            let balance_of_msg = build_message::<Erc20Ref>(
                contract_acc_id.clone()
            ).call(|erc20| erc20.balance_of(alice_acc_id.clone()));

            let balance_of_alice = client.call_dry_run(
                &ink_e2e::alice(),
                &balance_of_msg,
                0,
                None).await;

            assert_eq!(balance_of_alice.return_value(), 900);
            Ok(())
        }
    }
}
