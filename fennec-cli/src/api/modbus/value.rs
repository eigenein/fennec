use crate::prelude::*;

#[must_use]
#[derive(Copy, Clone, Debug)]
pub enum Value {
    U16(u16),
    I32(i32),
    U64(u64),
}

impl TryFrom<Value> for u16 {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::U16(value) => Ok(value),
            Value::I32(value) => Ok(value.try_into()?),
            Value::U64(value) => Ok(value.try_into()?),
        }
    }
}
