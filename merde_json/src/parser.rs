use jiter::{Jiter, JiterError, Peek};
use merde_types::{Array, CowStr, Map, Value};

pub(crate) fn bytes_to_value<'j>(src: &'j [u8]) -> Result<Value<'j>, JiterError> {
    let mut iter = Jiter::new(src);
    jiter_to_value(src, &mut iter)
}

pub(crate) fn jiter_to_value<'j>(
    src: &'j [u8],
    iter: &mut Jiter<'j>,
) -> Result<Value<'j>, JiterError> {
    let peek = iter.peek()?;
    jiter_to_value_with_peek(src, peek, iter)
}

pub(crate) fn jiter_to_value_with_peek<'j>(
    src: &'j [u8],
    peek: Peek,
    iter: &mut Jiter<'j>,
) -> Result<Value<'j>, JiterError> {
    Ok(match peek {
        Peek::Null => Value::Null,
        Peek::True => Value::Bool(true),
        Peek::False => Value::Bool(false),
        Peek::Minus => unimplemented!(),
        Peek::Infinity => Value::Float(f64::INFINITY),
        Peek::NaN => Value::Float(f64::NAN),
        Peek::String => {
            let s = iter.known_str()?;
            Value::Str(cowify(src, s))
        }
        Peek::Array => {
            let mut arr = Vec::new();
            let mut next = iter.known_array()?;
            while let Some(peek) = next {
                arr.push(jiter_to_value_with_peek(src, peek, iter)?);
                next = iter.array_step()?;
            }
            Value::Array(arr.into())
        }
        Peek::Object => {
            let mut obj = Map::new();
            let mut next = iter.known_object()?;
            while let Some(key) = next {
                let key = cowify(src, key);
                let value = jiter_to_value_with_peek(src, iter.peek()?, iter)?;
                obj.insert(key, value);
                next = iter.next_key()?;
            }
            Value::Map(obj.into())
        }
        p if p.is_num() => {
            if let Ok(i) = iter.next_int() {
                match i {
                    jiter::NumberInt::Int(i) => Value::Int(i),
                    jiter::NumberInt::BigInt(_) => {
                        unimplemented!("BigInt")
                    }
                }
            } else if let Ok(f) = iter.next_float() {
                Value::Float(f)
            } else {
                unreachable!("not an int, not a float!")
            }
        }
        _ => unimplemented!("peek {:?}", peek),
    })
}

fn cowify<'j>(src: &'j [u8], s: &str) -> CowStr<'j> {
    if src.as_ptr_range().contains(&s.as_ptr()) {
        CowStr::Borrowed(unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(s.as_ptr(), s.len()))
        })
    } else {
        CowStr::Owned(s.into())
    }
}

#[test]
fn test_cowify() {
    let src = "That's a subset!";
    let s = &src[4..8];
    assert_eq!(cowify(src.as_bytes(), s), CowStr::Borrowed(s));

    let src = "Not a subset";
    let s = "indeed not";
    assert_eq!(cowify(src.as_bytes(), s), CowStr::Owned(s.into()));
}

#[test]
fn test_jiter_to_value() {
    let src = r#"
    {
        "name": "John Doe",
        "age": 42,
        "address": {
            "street": "123 Main St",
            "city": "Anytown",
            "state": "CA",
            "zip": 12345
        },
        "friends": [
            "Alice",
            "Bob",
            "Charlie"
        ]
    }
    "#;

    let mut iter = Jiter::new(src.as_bytes());
    let value = jiter_to_value(src.as_bytes(), &mut iter).unwrap();
    assert_eq!(
        value,
        Value::Map(
            Map::new()
                .with("name", Value::Str(CowStr::from("John Doe")))
                .with("age", Value::Int(42))
                .with(
                    "address",
                    Map::new()
                        .with("street", Value::Str(CowStr::from("123 Main St")))
                        .with("city", Value::Str(CowStr::from("Anytown")))
                        .with("state", Value::Str(CowStr::from("CA")))
                        .with("zip", Value::Int(12345))
                )
                .with(
                    "friends",
                    Array::new()
                        .with(Value::from("Alice"))
                        .with(Value::from("Bob"))
                        .with(Value::from("Charlie"))
                )
        )
    );
}