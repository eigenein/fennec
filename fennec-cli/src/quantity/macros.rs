macro_rules! quantity {
    ($name:ident, $inner:tt, $unit:literal) => {
        new_type!($name, $inner);
        fmt!($name, $unit);

        impl $name {
            pub const fn zero() -> Self {
                Self(0 as $inner)
            }
        }
    };
}

macro_rules! new_type_base {
    ($name:ident, $inner:tt, #[$($derive:meta),*]) => {
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
            $($derive),*
        )]
        pub struct $name(pub $inner);
    };
}

macro_rules! new_type {
    ($name:ident,u16) => {
        new_type_base!($name, u16, #[
            ::std::cmp::PartialEq,
            ::std::cmp::Eq,
            ::std::cmp::PartialOrd,
            ::std::cmp::Ord
        ]);
    };
    ($name:ident,i64) => {
        new_type_base!($name, i64, #[
            ::std::cmp::PartialEq,
            ::std::cmp::Eq,
            ::std::cmp::PartialOrd,
            ::std::cmp::Ord,
            ::derive_more::Neg
        ]);
    };
    ($name:ident,f64) => {
        new_type_base!($name, f64, #[::derive_more::Neg]);
        ordered_float!($name);

        impl ::std::ops::Mul<f64> for $name {
            type Output = Self;

            fn mul(self, rhs: f64) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl ::std::ops::Div<Self> for $name {
            type Output = f64;

            fn div(self, rhs: Self) -> Self::Output {
                self.0 / rhs.0
            }
        }

        impl ::std::ops::Div<f64> for $name {
            type Output = Self;

            fn div(self, rhs: f64) -> Self::Output {
                Self(self.0 / rhs)
            }
        }
    };
}

macro_rules! fmt {
    ($name:ident, $unit:literal) => {
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
    };
}

macro_rules! ordered_float {
    ($name:path) => {
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

        impl ::std::cmp::Eq for $name {}
    };
}
