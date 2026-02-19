macro_rules! quantity {
    ($name:ident, $container:tt, $unit:literal) => {
        #[repr(transparent)]
        #[derive(
            ::derive_more::Add,
            ::derive_more::AddAssign,
            ::derive_more::FromStr,
            ::derive_more::Sub,
            ::derive_more::SubAssign,
            ::derive_more::Sum,
            ::serde::Deserialize,
            ::serde::Serialize,
            ::std::clone::Clone,
            ::std::marker::Copy,
        )]
        pub struct $name(pub $container);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, formatter)?;
                write!(formatter, " {}", $unit)
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Debug::fmt(&self.0, formatter)?;
                write!(formatter, "{}", $unit)
            }
        }

        impl $name {
            pub const fn zero() -> Self {
                Self(0 as $container)
            }
        }

        ordering!($name, $container);
    };
}

macro_rules! ordering {
    ($name:ty,f64) => {
        derive_neg!($name);
        ordered_float!($name);
    };
    ($name:ty,u16) => {
        derive_ordering!($name);
    };
    ($name:ty,i64) => {
        derive_neg!($name);
        derive_ordering!($name);
    };
}

macro_rules! ordered_float {
    ($name:ty) => {
        impl ::std::cmp::PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl ::std::cmp::Ord for $name {
            fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
                ::ordered_float::OrderedFloat(self.0).cmp(&::ordered_float::OrderedFloat(other.0))
            }
        }

        impl ::std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                ::ordered_float::OrderedFloat(self.0).eq(&::ordered_float::OrderedFloat(other.0))
            }
        }

        impl Eq for $name {}
    };
}

macro_rules! derive_neg {
    ($name:ty) => {
        impl ::std::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }
    };
}

macro_rules! derive_ordering {
    ($name:ty) => {
        impl ::std::cmp::PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl ::std::cmp::Ord for $name {
            fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl ::std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.eq(&other.0)
            }
        }

        impl ::std::cmp::Eq for $name {}
    };
}
