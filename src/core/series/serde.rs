use std::marker::PhantomData;

use serde::{
    Deserialize,
    Deserializer,
    Serialize,
    de::{DeserializeOwned, Error, Visitor},
    ser::SerializeMap,
};

use crate::core::series::Series;

impl<V: Serialize, I: Serialize> Serialize for Series<V, I> {
    fn serialize<S>(&self, serializer: S) -> crate::prelude::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut serialize_map = serializer.serialize_map(Some(self.0.len()))?;
        for (index, value) in &self.0 {
            serialize_map.serialize_entry(index, value)?;
        }
        serialize_map.end()
    }
}

impl<'de, V: DeserializeOwned, I: Ord + DeserializeOwned> Deserialize<'de> for Series<V, I> {
    fn deserialize<D>(deserializer: D) -> crate::prelude::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// Deserializes a series from a map.
        struct SeriesVisitor<V, I>(PhantomData<V>, PhantomData<I>);

        impl<'de, V: DeserializeOwned, I: Ord + DeserializeOwned> Visitor<'de> for SeriesVisitor<V, I> {
            type Value = Vec<(I, V)>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "an ordered map of indices to values")
            }

            fn visit_map<MA>(
                self,
                mut map_access: MA,
            ) -> crate::prelude::Result<Self::Value, MA::Error>
            where
                MA: serde::de::MapAccess<'de>,
            {
                let mut inner = Vec::with_capacity(map_access.size_hint().unwrap_or_default());
                while let Some(tuple) = map_access.next_entry()? {
                    inner.push(tuple);
                }
                Ok(inner)
            }
        }

        let this = Self(deserializer.deserialize_map(SeriesVisitor(PhantomData, PhantomData))?);
        match this.assert_sorted() {
            Ok(()) => Ok(this),
            Err(error) => Err(D::Error::custom(format!("the map is not sorted: {error:#}"))),
        }
    }
}
