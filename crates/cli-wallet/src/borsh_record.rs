use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use redb::{TypeName, Value};
use std::{any::type_name, fmt::Debug};

#[derive(Debug)]
pub struct BorshRecord<T>(pub T);

impl<T> Value for BorshRecord<T>
where
    T: Debug + BorshDeserialize + BorshSerialize,
{
    type SelfType<'a>
        = T
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        T::try_from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        to_vec(value).unwrap()
    }

    fn type_name() -> TypeName {
        TypeName::new(&format!("Borsh<{}>", type_name::<T>()))
    }
}
