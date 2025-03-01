//! Built-in implementations for common operators.

use super::call::FnCallArgs;
use super::native::FnBuiltin;
use crate::engine::OP_CONTAINS;
use crate::{Dynamic, ExclusiveRange, ImmutableString, InclusiveRange, INT};
use std::any::TypeId;
#[cfg(feature = "no_std")]
use std::prelude::v1::*;

#[cfg(not(feature = "no_float"))]
use crate::FLOAT;

#[cfg(not(feature = "no_float"))]
#[cfg(feature = "no_std")]
use num_traits::Float;

#[cfg(feature = "decimal")]
use rust_decimal::Decimal;

/// The message: data type was checked
const BUILTIN: &str = "data type was checked";

/// Is the type a numeric type?
#[inline]
#[must_use]
fn is_numeric(type_id: TypeId) -> bool {
    let result = false;

    #[cfg(not(feature = "only_i64"))]
    #[cfg(not(feature = "only_i32"))]
    let result = result
        || type_id == TypeId::of::<u8>()
        || type_id == TypeId::of::<u16>()
        || type_id == TypeId::of::<u32>()
        || type_id == TypeId::of::<u64>()
        || type_id == TypeId::of::<i8>()
        || type_id == TypeId::of::<i16>()
        || type_id == TypeId::of::<i32>()
        || type_id == TypeId::of::<i64>();

    #[cfg(not(feature = "only_i64"))]
    #[cfg(not(feature = "only_i32"))]
    #[cfg(not(target_family = "wasm"))]
    let result = result || type_id == TypeId::of::<u128>() || type_id == TypeId::of::<i128>();

    #[cfg(not(feature = "no_float"))]
    let result = result || type_id == TypeId::of::<f32>() || type_id == TypeId::of::<f64>();

    #[cfg(feature = "decimal")]
    let result = result || type_id == TypeId::of::<rust_decimal::Decimal>();

    result
}

/// Build in common binary operator implementations to avoid the cost of calling a registered function.
///
/// The return function will be registered as a _method_, so the first parameter cannot be consumed.
#[must_use]
pub fn get_builtin_binary_op_fn(op: &str, x: &Dynamic, y: &Dynamic) -> Option<FnBuiltin> {
    let type1 = x.type_id();
    let type2 = y.type_id();

    let types_pair = (type1, type2);

    macro_rules! impl_op {
        ($xx:ident $op:tt $yy:ident) => { |_, args| {
            let x = &*args[0].read_lock::<$xx>().expect(BUILTIN);
            let y = &*args[1].read_lock::<$yy>().expect(BUILTIN);
            Ok((x $op y).into())
        } };
        ($xx:ident . $func:ident ( $yy:ty )) => { |_, args| {
            let x = &*args[0].read_lock::<$xx>().expect(BUILTIN);
            let y = &*args[1].read_lock::<$yy>().expect(BUILTIN);
            Ok(x.$func(y).into())
        } };
        ($xx:ident . $func:ident ( $yy:ident . $yyy:ident () )) => { |_, args| {
            let x = &*args[0].read_lock::<$xx>().expect(BUILTIN);
            let y = &*args[1].read_lock::<$yy>().expect(BUILTIN);
            Ok(x.$func(y.$yyy()).into())
        } };
        ($func:ident ( $op:tt )) => { |_, args| {
            let (x, y) = $func(args);
            Ok((x $op y).into())
        } };
        ($base:ty => $xx:ident $op:tt $yy:ident) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN) as $base;
            let y = args[1].$yy().expect(BUILTIN) as $base;
            Ok((x $op y).into())
        } };
        ($base:ty => $xx:ident . $func:ident ( $yy:ident as $yyy:ty)) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN) as $base;
            let y = args[1].$yy().expect(BUILTIN) as $base;
            Ok(x.$func(y as $yyy).into())
        } };
        ($base:ty => $func:ident ( $xx:ident, $yy:ident )) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN) as $base;
            let y = args[1].$yy().expect(BUILTIN) as $base;
            $func(x, y).map(Into::into)
        } };
        (from $base:ty => $xx:ident $op:tt $yy:ident) => { |_, args| {
            let x = <$base>::from(args[0].$xx().expect(BUILTIN));
            let y = <$base>::from(args[1].$yy().expect(BUILTIN));
            Ok((x $op y).into())
        } };
        (from $base:ty => $xx:ident . $func:ident ( $yy:ident )) => { |_, args| {
            let x = <$base>::from(args[0].$xx().expect(BUILTIN));
            let y = <$base>::from(args[1].$yy().expect(BUILTIN));
            Ok(x.$func(y).into())
        } };
        (from $base:ty => $func:ident ( $xx:ident, $yy:ident )) => { |_, args| {
            let x = <$base>::from(args[0].$xx().expect(BUILTIN));
            let y = <$base>::from(args[1].$yy().expect(BUILTIN));
            $func(x, y).map(Into::into)
        } };
    }

    // Check for common patterns
    if type1 == type2 {
        if type1 == TypeId::of::<INT>() {
            #[cfg(not(feature = "unchecked"))]
            use crate::packages::arithmetic::arith_basic::INT::functions::*;

            #[cfg(not(feature = "unchecked"))]
            match op {
                "+" => return Some(impl_op!(INT => add(as_int, as_int))),
                "-" => return Some(impl_op!(INT => subtract(as_int, as_int))),
                "*" => return Some(impl_op!(INT => multiply(as_int, as_int))),
                "/" => return Some(impl_op!(INT => divide(as_int, as_int))),
                "%" => return Some(impl_op!(INT => modulo(as_int, as_int))),
                "**" => return Some(impl_op!(INT => power(as_int, as_int))),
                ">>" => return Some(impl_op!(INT => shift_right(as_int, as_int))),
                "<<" => return Some(impl_op!(INT => shift_left(as_int, as_int))),
                _ => (),
            }

            #[cfg(feature = "unchecked")]
            match op {
                "+" => return Some(impl_op!(INT => as_int + as_int)),
                "-" => return Some(impl_op!(INT => as_int - as_int)),
                "*" => return Some(impl_op!(INT => as_int * as_int)),
                "/" => return Some(impl_op!(INT => as_int / as_int)),
                "%" => return Some(impl_op!(INT => as_int % as_int)),
                "**" => return Some(impl_op!(INT => as_int.pow(as_int as u32))),
                ">>" => return Some(impl_op!(INT => as_int >> as_int)),
                "<<" => return Some(impl_op!(INT => as_int << as_int)),
                _ => (),
            }

            return match op {
                "==" => Some(impl_op!(INT => as_int == as_int)),
                "!=" => Some(impl_op!(INT => as_int != as_int)),
                ">" => Some(impl_op!(INT => as_int > as_int)),
                ">=" => Some(impl_op!(INT => as_int >= as_int)),
                "<" => Some(impl_op!(INT => as_int < as_int)),
                "<=" => Some(impl_op!(INT => as_int <= as_int)),
                "&" => Some(impl_op!(INT => as_int & as_int)),
                "|" => Some(impl_op!(INT => as_int | as_int)),
                "^" => Some(impl_op!(INT => as_int ^ as_int)),
                ".." => Some(|_, args| {
                    let x = args[0].as_int().expect(BUILTIN);
                    let y = args[1].as_int().expect(BUILTIN);
                    Ok((x..y).into())
                }),
                "..=" => Some(|_, args| {
                    let x = args[0].as_int().expect(BUILTIN);
                    let y = args[1].as_int().expect(BUILTIN);
                    Ok((x..=y).into())
                }),
                _ => None,
            };
        }

        if type1 == TypeId::of::<bool>() {
            return match op {
                "==" => Some(impl_op!(bool => as_bool == as_bool)),
                "!=" => Some(impl_op!(bool => as_bool != as_bool)),
                ">" => Some(impl_op!(bool => as_bool > as_bool)),
                ">=" => Some(impl_op!(bool => as_bool >= as_bool)),
                "<" => Some(impl_op!(bool => as_bool < as_bool)),
                "<=" => Some(impl_op!(bool => as_bool <= as_bool)),
                "&" => Some(impl_op!(bool => as_bool & as_bool)),
                "|" => Some(impl_op!(bool => as_bool | as_bool)),
                "^" => Some(impl_op!(bool => as_bool ^ as_bool)),
                _ => None,
            };
        }

        if type1 == TypeId::of::<ImmutableString>() {
            return match op {
                "+" => Some(impl_op!(ImmutableString + ImmutableString)),
                "-" => Some(impl_op!(ImmutableString - ImmutableString)),
                "==" => Some(impl_op!(ImmutableString == ImmutableString)),
                "!=" => Some(impl_op!(ImmutableString != ImmutableString)),
                ">" => Some(impl_op!(ImmutableString > ImmutableString)),
                ">=" => Some(impl_op!(ImmutableString >= ImmutableString)),
                "<" => Some(impl_op!(ImmutableString < ImmutableString)),
                "<=" => Some(impl_op!(ImmutableString <= ImmutableString)),
                OP_CONTAINS => Some(impl_op!(ImmutableString.contains(ImmutableString.as_str()))),
                _ => None,
            };
        }

        if type1 == TypeId::of::<char>() {
            return match op {
                "+" => Some(|_, args| {
                    let x = args[0].as_char().expect(BUILTIN);
                    let y = args[1].as_char().expect(BUILTIN);
                    Ok(format!("{x}{y}").into())
                }),
                "==" => Some(impl_op!(char => as_char == as_char)),
                "!=" => Some(impl_op!(char => as_char != as_char)),
                ">" => Some(impl_op!(char => as_char > as_char)),
                ">=" => Some(impl_op!(char => as_char >= as_char)),
                "<" => Some(impl_op!(char => as_char < as_char)),
                "<=" => Some(impl_op!(char => as_char <= as_char)),
                _ => None,
            };
        }

        #[cfg(not(feature = "no_index"))]
        if type1 == TypeId::of::<crate::Blob>() {
            use crate::Blob;

            return match op {
                "+" => Some(|_, args| {
                    let blob1 = &*args[0].read_lock::<Blob>().expect(BUILTIN);
                    let blob2 = &*args[1].read_lock::<Blob>().expect(BUILTIN);

                    Ok(Dynamic::from_blob(if blob2.is_empty() {
                        blob1.clone()
                    } else if blob1.is_empty() {
                        blob2.clone()
                    } else {
                        let mut blob = blob1.clone();
                        blob.extend(blob2);
                        blob
                    }))
                }),
                "==" => Some(impl_op!(Blob == Blob)),
                "!=" => Some(impl_op!(Blob != Blob)),
                _ => None,
            };
        }

        if type1 == TypeId::of::<()>() {
            return match op {
                "==" => Some(|_, _| Ok(Dynamic::TRUE)),
                "!=" | ">" | ">=" | "<" | "<=" => Some(|_, _| Ok(Dynamic::FALSE)),
                _ => None,
            };
        }
    }

    #[cfg(not(feature = "no_float"))]
    macro_rules! impl_float {
        ($x:ty, $xx:ident, $y:ty, $yy:ident) => {
            if types_pair == (TypeId::of::<$x>(), TypeId::of::<$y>()) {
                return match op {
                    "+" => Some(impl_op!(FLOAT => $xx + $yy)),
                    "-" => Some(impl_op!(FLOAT => $xx - $yy)),
                    "*" => Some(impl_op!(FLOAT => $xx * $yy)),
                    "/" => Some(impl_op!(FLOAT => $xx / $yy)),
                    "%" => Some(impl_op!(FLOAT => $xx % $yy)),
                    "**" => Some(impl_op!(FLOAT => $xx.powf($yy as FLOAT))),
                    "==" => Some(impl_op!(FLOAT => $xx == $yy)),
                    "!=" => Some(impl_op!(FLOAT => $xx != $yy)),
                    ">" => Some(impl_op!(FLOAT => $xx > $yy)),
                    ">=" => Some(impl_op!(FLOAT => $xx >= $yy)),
                    "<" => Some(impl_op!(FLOAT => $xx < $yy)),
                    "<=" => Some(impl_op!(FLOAT => $xx <= $yy)),
                    _ => None,
                };
            }
        };
    }

    #[cfg(not(feature = "no_float"))]
    {
        impl_float!(FLOAT, as_float, FLOAT, as_float);
        impl_float!(FLOAT, as_float, INT, as_int);
        impl_float!(INT, as_int, FLOAT, as_float);
    }

    #[cfg(feature = "decimal")]
    macro_rules! impl_decimal {
        ($x:ty, $xx:ident, $y:ty, $yy:ident) => {
            if types_pair == (TypeId::of::<$x>(), TypeId::of::<$y>()) {
                #[cfg(not(feature = "unchecked"))]
                use crate::packages::arithmetic::decimal_functions::*;

                #[cfg(not(feature = "unchecked"))]
                match op {
                    "+" => return Some(impl_op!(from Decimal => add($xx, $yy))),
                    "-" => return Some(impl_op!(from Decimal => subtract($xx, $yy))),
                    "*" => return Some(impl_op!(from Decimal => multiply($xx, $yy))),
                    "/" => return Some(impl_op!(from Decimal => divide($xx, $yy))),
                    "%" => return Some(impl_op!(from Decimal => modulo($xx, $yy))),
                    "**" => return Some(impl_op!(from Decimal => power($xx, $yy))),
                    _ => ()
                }

                #[cfg(feature = "unchecked")]
                use rust_decimal::MathematicalOps;

                #[cfg(feature = "unchecked")]
                match op {
                    "+" => return Some(impl_op!(from Decimal => $xx + $yy)),
                    "-" => return Some(impl_op!(from Decimal => $xx - $yy)),
                    "*" => return Some(impl_op!(from Decimal => $xx * $yy)),
                    "/" => return Some(impl_op!(from Decimal => $xx / $yy)),
                    "%" => return Some(impl_op!(from Decimal => $xx % $yy)),
                    "**" => return Some(impl_op!(from Decimal => $xx.powd($yy))),
                    _ => ()
                }

                return match op {
                    "==" => Some(impl_op!(from Decimal => $xx == $yy)),
                    "!=" => Some(impl_op!(from Decimal => $xx != $yy)),
                    ">" => Some(impl_op!(from Decimal => $xx > $yy)),
                    ">=" => Some(impl_op!(from Decimal => $xx >= $yy)),
                    "<" => Some(impl_op!(from Decimal => $xx < $yy)),
                    "<=" => Some(impl_op!(from Decimal => $xx <= $yy)),
                    _ =>  None
                };
            }
        };
    }

    #[cfg(feature = "decimal")]
    {
        impl_decimal!(Decimal, as_decimal, Decimal, as_decimal);
        impl_decimal!(Decimal, as_decimal, INT, as_int);
        impl_decimal!(INT, as_int, Decimal, as_decimal);
    }

    // char op string
    if types_pair == (TypeId::of::<char>(), TypeId::of::<ImmutableString>()) {
        fn get_s1s2(args: &FnCallArgs) -> ([char; 2], [char; 2]) {
            let x = args[0].as_char().expect(BUILTIN);
            let y = &*args[1].read_lock::<ImmutableString>().expect(BUILTIN);
            let s1 = [x, '\0'];
            let mut y = y.chars();
            let s2 = [y.next().unwrap_or('\0'), y.next().unwrap_or('\0')];
            (s1, s2)
        }

        return match op {
            "+" => Some(|_, args| {
                let x = args[0].as_char().expect(BUILTIN);
                let y = &*args[1].read_lock::<ImmutableString>().expect(BUILTIN);
                Ok(format!("{x}{y}").into())
            }),
            "==" => Some(impl_op!(get_s1s2(==))),
            "!=" => Some(impl_op!(get_s1s2(!=))),
            ">" => Some(impl_op!(get_s1s2(>))),
            ">=" => Some(impl_op!(get_s1s2(>=))),
            "<" => Some(impl_op!(get_s1s2(<))),
            "<=" => Some(impl_op!(get_s1s2(<=))),
            _ => None,
        };
    }
    // string op char
    if types_pair == (TypeId::of::<ImmutableString>(), TypeId::of::<char>()) {
        fn get_s1s2(args: &FnCallArgs) -> ([char; 2], [char; 2]) {
            let x = &*args[0].read_lock::<ImmutableString>().expect(BUILTIN);
            let y = args[1].as_char().expect(BUILTIN);
            let mut x = x.chars();
            let s1 = [x.next().unwrap_or('\0'), x.next().unwrap_or('\0')];
            let s2 = [y, '\0'];
            (s1, s2)
        }

        return match op {
            "+" => Some(|_, args| {
                let x = &*args[0].read_lock::<ImmutableString>().expect(BUILTIN);
                let y = args[1].as_char().expect(BUILTIN);
                Ok((x + y).into())
            }),
            "-" => Some(|_, args| {
                let x = &*args[0].read_lock::<ImmutableString>().expect(BUILTIN);
                let y = args[1].as_char().expect(BUILTIN);
                Ok((x - y).into())
            }),
            "==" => Some(impl_op!(get_s1s2(==))),
            "!=" => Some(impl_op!(get_s1s2(!=))),
            ">" => Some(impl_op!(get_s1s2(>))),
            ">=" => Some(impl_op!(get_s1s2(>=))),
            "<" => Some(impl_op!(get_s1s2(<))),
            "<=" => Some(impl_op!(get_s1s2(<=))),
            OP_CONTAINS => Some(|_, args| {
                let s = &*args[0].read_lock::<ImmutableString>().expect(BUILTIN);
                let c = args[1].as_char().expect(BUILTIN);
                Ok(s.contains(c).into())
            }),
            _ => None,
        };
    }
    // () op string
    if types_pair == (TypeId::of::<()>(), TypeId::of::<ImmutableString>()) {
        return match op {
            "+" => Some(|_, args| Ok(args[1].clone())),
            "==" | ">" | ">=" | "<" | "<=" => Some(|_, _| Ok(Dynamic::FALSE)),
            "!=" => Some(|_, _| Ok(Dynamic::TRUE)),
            _ => None,
        };
    }
    // string op ()
    if types_pair == (TypeId::of::<ImmutableString>(), TypeId::of::<()>()) {
        return match op {
            "+" => Some(|_, args| Ok(args[0].clone())),
            "==" | ">" | ">=" | "<" | "<=" => Some(|_, _| Ok(Dynamic::FALSE)),
            "!=" => Some(|_, _| Ok(Dynamic::TRUE)),
            _ => None,
        };
    }

    // blob
    #[cfg(not(feature = "no_index"))]
    if type1 == TypeId::of::<crate::Blob>() {
        use crate::Blob;

        if type2 == TypeId::of::<INT>() {
            return match op {
                OP_CONTAINS => Some(|_, args| {
                    let blob = &*args[0].read_lock::<Blob>().expect(BUILTIN);
                    let x = (args[1].as_int().expect("`INT`") & 0x0000_00ff) as u8;
                    Ok((!blob.is_empty() && blob.contains(&x)).into())
                }),
                _ => None,
            };
        }
        if type2 == TypeId::of::<char>() {
            return match op {
                "+" => Some(|_, args| {
                    let mut buf = [0_u8; 4];
                    let mut blob = args[0].read_lock::<Blob>().expect(BUILTIN).clone();
                    let x = args[1].as_char().expect("`char`").encode_utf8(&mut buf);
                    blob.extend(x.as_bytes());
                    Ok(Dynamic::from_blob(blob))
                }),
                _ => None,
            };
        }
    }

    // map op string
    #[cfg(not(feature = "no_object"))]
    if types_pair == (TypeId::of::<crate::Map>(), TypeId::of::<ImmutableString>()) {
        use crate::Map;

        return match op {
            OP_CONTAINS => Some(impl_op!(Map.contains_key(ImmutableString.as_str()))),
            _ => None,
        };
    }

    // Non-compatible ranges
    if types_pair
        == (
            TypeId::of::<ExclusiveRange>(),
            TypeId::of::<InclusiveRange>(),
        )
        || types_pair
            == (
                TypeId::of::<InclusiveRange>(),
                TypeId::of::<ExclusiveRange>(),
            )
    {
        return match op {
            "!=" => Some(|_, _| Ok(Dynamic::TRUE)),
            "==" => Some(|_, _| Ok(Dynamic::FALSE)),
            _ => None,
        };
    }

    // Handle ranges here because ranges are implemented as custom type
    if type1 == TypeId::of::<ExclusiveRange>() {
        if type2 == TypeId::of::<INT>() {
            return match op {
                OP_CONTAINS => Some(|_, args| {
                    let range = &*args[0].read_lock::<ExclusiveRange>().expect(BUILTIN);
                    let x = args[1].as_int().expect("`INT`");
                    Ok(range.contains(&x).into())
                }),
                _ => None,
            };
        }
        if type1 == type2 {
            return match op {
                "==" => Some(impl_op!(ExclusiveRange == ExclusiveRange)),
                "!=" => Some(impl_op!(ExclusiveRange != ExclusiveRange)),
                _ => None,
            };
        }
    }

    if type1 == TypeId::of::<InclusiveRange>() {
        if type2 == TypeId::of::<INT>() {
            return match op {
                OP_CONTAINS => Some(|_, args| {
                    let range = &*args[0].read_lock::<InclusiveRange>().expect(BUILTIN);
                    let x = args[1].as_int().expect("`INT`");
                    Ok(range.contains(&x).into())
                }),
                _ => None,
            };
        }
        if type1 == type2 {
            return match op {
                "==" => Some(impl_op!(InclusiveRange == InclusiveRange)),
                "!=" => Some(impl_op!(InclusiveRange != InclusiveRange)),
                _ => None,
            };
        }
    }

    // One of the operands is a custom type, so it is never built-in
    if x.is_variant() || y.is_variant() {
        return if is_numeric(type1) && is_numeric(type2) {
            // Disallow comparisons between different numeric types
            None
        } else if type1 != type2 {
            // If the types are not the same, default to not compare
            match op {
                "!=" => Some(|_, _| Ok(Dynamic::TRUE)),
                "==" | ">" | ">=" | "<" | "<=" => Some(|_, _| Ok(Dynamic::FALSE)),
                _ => None,
            }
        } else {
            // Disallow comparisons between the same type
            None
        };
    }

    // Default comparison operators for different types
    if type2 != type1 {
        return match op {
            "!=" => Some(|_, _| Ok(Dynamic::TRUE)),
            "==" | ">" | ">=" | "<" | "<=" => Some(|_, _| Ok(Dynamic::FALSE)),
            _ => None,
        };
    }

    // Beyond here, type1 == type2
    None
}

/// Build in common operator assignment implementations to avoid the cost of calling a registered function.
///
/// The return function is registered as a _method_, so the first parameter cannot be consumed.
#[must_use]
pub fn get_builtin_op_assignment_fn(op: &str, x: &Dynamic, y: &Dynamic) -> Option<FnBuiltin> {
    let type1 = x.type_id();
    let type2 = y.type_id();

    let types_pair = (type1, type2);

    macro_rules! impl_op {
        ($x:ty = x $op:tt $yy:ident) => { |_, args| {
            let x = args[0].$yy().expect(BUILTIN);
            let y = args[1].$yy().expect(BUILTIN) as $x;
            Ok((*args[0].write_lock::<$x>().expect(BUILTIN) = x $op y).into())
        } };
        ($x:ident $op:tt $yy:ident) => { |_, args| {
            let y = args[1].$yy().expect(BUILTIN) as $x;
            Ok((*args[0].write_lock::<$x>().expect(BUILTIN) $op y).into())
        } };
        ($x:ident $op:tt $yy:ident as $yyy:ty) => { |_, args| {
            let y = args[1].$yy().expect(BUILTIN) as $yyy;
            Ok((*args[0].write_lock::<$x>().expect(BUILTIN) $op y).into())
        } };
        ($x:ty => $xx:ident . $func:ident ( $yy:ident as $yyy:ty )) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN);
            let y = args[1].$yy().expect(BUILTIN) as $x;
            Ok((*args[0].write_lock::<$x>().expect(BUILTIN) = x.$func(y as $yyy)).into())
        } };
        ($x:ty => $func:ident ( $xx:ident, $yy:ident )) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN);
            let y = args[1].$yy().expect(BUILTIN) as $x;
            Ok((*args[0].write_lock().expect(BUILTIN) = $func(x, y)?).into())
        } };
        (from $x:ident $op:tt $yy:ident) => { |_, args| {
            let y = <$x>::from(args[1].$yy().expect(BUILTIN));
            Ok((*args[0].write_lock::<$x>().expect(BUILTIN) $op y).into())
        } };
        (from $x:ty => $xx:ident . $func:ident ( $yy:ident )) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN);
            let y = <$x>::from(args[1].$yy().expect(BUILTIN));
            Ok((*args[0].write_lock::<$x>().expect(BUILTIN) = x.$func(y)).into())
        } };
        (from $x:ty => $func:ident ( $xx:ident, $yy:ident )) => { |_, args| {
            let x = args[0].$xx().expect(BUILTIN);
            let y = <$x>::from(args[1].$yy().expect(BUILTIN));
            Ok((*args[0].write_lock().expect(BUILTIN) = $func(x, y)?).into())
        } };
    }

    // Check for common patterns
    if type1 == type2 {
        if type1 == TypeId::of::<INT>() {
            #[cfg(not(feature = "unchecked"))]
            use crate::packages::arithmetic::arith_basic::INT::functions::*;

            #[cfg(not(feature = "unchecked"))]
            match op {
                "+=" => return Some(impl_op!(INT => add(as_int, as_int))),
                "-=" => return Some(impl_op!(INT => subtract(as_int, as_int))),
                "*=" => return Some(impl_op!(INT => multiply(as_int, as_int))),
                "/=" => return Some(impl_op!(INT => divide(as_int, as_int))),
                "%=" => return Some(impl_op!(INT => modulo(as_int, as_int))),
                "**=" => return Some(impl_op!(INT => power(as_int, as_int))),
                ">>=" => return Some(impl_op!(INT => shift_right(as_int, as_int))),
                "<<=" => return Some(impl_op!(INT => shift_left(as_int, as_int))),
                _ => (),
            }

            #[cfg(feature = "unchecked")]
            match op {
                "+=" => return Some(impl_op!(INT += as_int)),
                "-=" => return Some(impl_op!(INT -= as_int)),
                "*=" => return Some(impl_op!(INT *= as_int)),
                "/=" => return Some(impl_op!(INT /= as_int)),
                "%=" => return Some(impl_op!(INT %= as_int)),
                "**=" => return Some(impl_op!(INT => as_int.pow(as_int as u32))),
                ">>=" => return Some(impl_op!(INT >>= as_int)),
                "<<=" => return Some(impl_op!(INT <<= as_int)),
                _ => (),
            }

            return match op {
                "&=" => Some(impl_op!(INT &= as_int)),
                "|=" => Some(impl_op!(INT |= as_int)),
                "^=" => Some(impl_op!(INT ^= as_int)),
                _ => None,
            };
        }

        if type1 == TypeId::of::<bool>() {
            return match op {
                "&=" => Some(impl_op!(bool = x && as_bool)),
                "|=" => Some(impl_op!(bool = x || as_bool)),
                _ => None,
            };
        }

        if type1 == TypeId::of::<char>() {
            return match op {
                "+=" => Some(|_, args| {
                    let y = args[1].as_char().expect(BUILTIN);
                    let x = &mut *args[0].write_lock::<Dynamic>().expect(BUILTIN);
                    Ok((*x = format!("{x}{y}").into()).into())
                }),
                _ => None,
            };
        }

        if type1 == TypeId::of::<ImmutableString>() {
            return match op {
                "+=" => Some(|_, args| {
                    let (first, second) = args.split_first_mut().expect(BUILTIN);
                    let x = &mut *first.write_lock::<ImmutableString>().expect(BUILTIN);
                    let y = std::mem::take(second[0]).cast::<ImmutableString>();
                    Ok((*x += y).into())
                }),
                "-=" => Some(|_, args| {
                    let (first, second) = args.split_first_mut().expect(BUILTIN);
                    let x = &mut *first.write_lock::<ImmutableString>().expect(BUILTIN);
                    let y = std::mem::take(second[0]).cast::<ImmutableString>();
                    Ok((*x -= y).into())
                }),
                _ => None,
            };
        }

        #[cfg(not(feature = "no_index"))]
        if type1 == TypeId::of::<crate::Blob>() {
            use crate::Blob;

            return match op {
                "+=" => Some(|_, args| {
                    let blob2 = std::mem::take(args[1]).cast::<Blob>();
                    let blob1 = &mut *args[0].write_lock::<Blob>().expect(BUILTIN);
                    Ok(crate::packages::blob_basic::blob_functions::append(blob1, blob2).into())
                }),
                _ => None,
            };
        }
    }

    #[cfg(not(feature = "no_float"))]
    macro_rules! impl_float {
        ($x:ident, $xx:ident, $y:ty, $yy:ident) => {
            if types_pair == (TypeId::of::<$x>(), TypeId::of::<$y>()) {
                return match op {
                    "+=" => Some(impl_op!($x += $yy)),
                    "-=" => Some(impl_op!($x -= $yy)),
                    "*=" => Some(impl_op!($x *= $yy)),
                    "/=" => Some(impl_op!($x /= $yy)),
                    "%=" => Some(impl_op!($x %= $yy)),
                    "**=" => Some(impl_op!($x => $xx.powf($yy as $x))),
                    _ => None,
                };
            }
        }
    }

    #[cfg(not(feature = "no_float"))]
    {
        impl_float!(FLOAT, as_float, FLOAT, as_float);
        impl_float!(FLOAT, as_float, INT, as_int);
    }

    #[cfg(feature = "decimal")]
    macro_rules! impl_decimal {
        ($x:ident, $xx:ident, $y:ty, $yy:ident) => {
            if types_pair == (TypeId::of::<$x>(), TypeId::of::<$y>()) {
                #[cfg(not(feature = "unchecked"))]
                use crate::packages::arithmetic::decimal_functions::*;

                #[cfg(not(feature = "unchecked"))]
                return match op {
                    "+=" => Some(impl_op!(from $x => add($xx, $yy))),
                    "-=" => Some(impl_op!(from $x => subtract($xx, $yy))),
                    "*=" => Some(impl_op!(from $x => multiply($xx, $yy))),
                    "/=" => Some(impl_op!(from $x => divide($xx, $yy))),
                    "%=" => Some(impl_op!(from $x => modulo($xx, $yy))),
                    "**=" => Some(impl_op!(from $x => power($xx, $yy))),
                    _ => None,
                };

                #[cfg(feature = "unchecked")]
                use rust_decimal::MathematicalOps;

                #[cfg(feature = "unchecked")]
                return match op {
                    "+=" => Some(impl_op!(from $x += $yy)),
                    "-=" => Some(impl_op!(from $x -= $yy)),
                    "*=" => Some(impl_op!(from $x *= $yy)),
                    "/=" => Some(impl_op!(from $x /= $yy)),
                    "%=" => Some(impl_op!(from $x %= $yy)),
                    "**=" => Some(impl_op!(from $x => $xx.powd($yy))),
                    _ =>  None,
                };
            }
        };
    }

    #[cfg(feature = "decimal")]
    {
        impl_decimal!(Decimal, as_decimal, Decimal, as_decimal);
        impl_decimal!(Decimal, as_decimal, INT, as_int);
    }

    // string op= char
    if types_pair == (TypeId::of::<ImmutableString>(), TypeId::of::<char>()) {
        return match op {
            "+=" => Some(impl_op!(ImmutableString += as_char as char)),
            "-=" => Some(impl_op!(ImmutableString -= as_char as char)),
            _ => None,
        };
    }
    // char op= string
    if types_pair == (TypeId::of::<char>(), TypeId::of::<ImmutableString>()) {
        return match op {
            "+=" => Some(|_, args| {
                let mut ch = args[0].as_char().expect(BUILTIN).to_string();
                ch.push_str(
                    args[1]
                        .read_lock::<ImmutableString>()
                        .expect(BUILTIN)
                        .as_str(),
                );

                let mut x = args[0].write_lock::<Dynamic>().expect(BUILTIN);
                Ok((*x = ch.into()).into())
            }),
            _ => None,
        };
    }

    // array op= any
    #[cfg(not(feature = "no_index"))]
    if type1 == TypeId::of::<crate::Array>() {
        use crate::packages::array_basic::array_functions::*;
        use crate::Array;

        if type2 == TypeId::of::<crate::Array>() {
            return match op {
                "+=" => Some(|_, args| {
                    let array2 = std::mem::take(args[1]).cast::<Array>();
                    let array1 = &mut *args[0].write_lock::<Array>().expect(BUILTIN);
                    Ok(append(array1, array2).into())
                }),
                _ => None,
            };
        }
        return match op {
            "+=" => Some(|_, args| {
                let x = std::mem::take(args[1]);
                let array = &mut *args[0].write_lock::<Array>().expect(BUILTIN);
                Ok(push(array, x).into())
            }),
            _ => None,
        };
    }

    #[cfg(not(feature = "no_index"))]
    {
        use crate::Blob;

        // blob op= int
        if types_pair == (TypeId::of::<Blob>(), TypeId::of::<INT>()) {
            return match op {
                "+=" => Some(|_, args| {
                    let x = args[1].as_int().expect("`INT`");
                    let blob = &mut *args[0].write_lock::<Blob>().expect(BUILTIN);
                    Ok(crate::packages::blob_basic::blob_functions::push(blob, x).into())
                }),
                _ => None,
            };
        }

        // blob op= char
        if types_pair == (TypeId::of::<Blob>(), TypeId::of::<char>()) {
            return match op {
                "+=" => Some(|_, args| {
                    let x = args[1].as_char().expect("`char`");
                    let blob = &mut *args[0].write_lock::<Blob>().expect(BUILTIN);
                    Ok(crate::packages::blob_basic::blob_functions::append_char(blob, x).into())
                }),
                _ => None,
            };
        }

        // blob op= string
        if types_pair == (TypeId::of::<Blob>(), TypeId::of::<ImmutableString>()) {
            return match op {
                "+=" => Some(|_, args| {
                    let s = std::mem::take(args[1]).cast::<ImmutableString>();
                    let blob = &mut *args[0].write_lock::<Blob>().expect(BUILTIN);
                    Ok(crate::packages::blob_basic::blob_functions::append_str(blob, &s).into())
                }),
                _ => None,
            };
        }
    }

    None
}
