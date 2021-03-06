use std::collections::HashMap;
use std::error;
use std::error::Error as _StdError;
use std::fmt;

use serde;

use compiler::{Resolve, ResolveContext};
use compiler;
use data;
use parser;

use self::LocalizeError::*;

/*
/// The current L20n context.
pub struct Context {
    locales: HashMap<String, Locale>,
    default_locale: String
}

impl Context {
    /// Creates a new Context, using `i-default` as the default locale.
    pub fn new() -> Context {
        Context::with_default(String::from("i-default"))
    }

    /// Create a new Context with the specified default locale.
    pub fn with_default<S: Into<String>>(locale: S) -> Context {
        let mut locales = HashMap::new();
        locales.insert(locale.into(), Locale::new());
        Context {
            locales: locales,
            default_locale: locale
        }
    }
    pub fn add_resource(&mut self, res: String) -> Result<(), parser::ParseError>{
        self.add_locale_resource("i-default".to_string(), res) //self.default_locale.clone(), res)
    }


    pub fn add_locale_resource(&mut self, name: String, res: String) -> Result<(), parser::ParseError> {
        let mut locale = self.locales.find_or_insert_with(name, |_| Locale::new());
        let entities = try!(compiler::compile(res.as_slice()));
        locale.resources.extend(entities.move_iter());
        Ok(())
    }

    pub fn locale<'a>(&'a self) -> &'a Locale {
        self.get_locale(self.default_locale.as_slice()).unwrap()
    }

    pub fn get_locale<'a>(&'a self, name: &str) -> Option<&'a Locale> {
        self.locales.find_equiv(&name)
    }

}

*/

/// A Locale contains all the resources for a specific language.
pub struct Locale {
    resources: HashMap<String, parser::Entry>
}

/// An enum of the various errors that can occur during localization.
#[derive(Debug)]
pub enum LocalizeError {
    /// Wraps a DecodeError.
    DecodeError(::serde::de::value::Error),
    /// Wraps an EncodeError.
    EncodeError(data::EncodeError),
    /// Wraps a ResolveError.
    ResolveError(compiler::ResolveError)
}

impl error::Error for LocalizeError {
    fn description(&self) -> &str {
        match *self {
            LocalizeError::DecodeError(_) => "Decode error",
            LocalizeError::EncodeError(_) => "Encode error",
            LocalizeError::ResolveError(_) => "Resolve error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            LocalizeError::DecodeError(_) => None, // @FIXME
            LocalizeError::EncodeError(ref err) => Some(err),
            LocalizeError::ResolveError(ref err) => Some(err),
        }
    }
}

impl fmt::Display for LocalizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LocalizeError::DecodeError(_) => write!(f, "{}", self.description()), // @FIXME
            LocalizeError::EncodeError(ref err) => write!(f, "{}: {}", self.description(), err),
            LocalizeError::ResolveError(ref err) => write!(f, "{}: {}", self.description(), err),
        }
    }
}

/// A Result of trying to localize.
pub type LocalizeResult<T> = Result<T, LocalizeError>;

impl Locale {

    /// Creates a new empty Locale.
    pub fn new() -> Locale {
        Locale {
            resources: HashMap::new()
        }
    }

    /// Add a L20n string resource, and it will be parsed.
    pub fn add_resource(&mut self, res: &str) -> Result<(), parser::ParseError> {
        let entities = try!(compiler::compile(res));
        self.resources.extend(entities.into_iter());
        Ok(())
    }

    /// Resolves all the resouces into Strings, and returns a Deserialize
    /// object of your choosing.
    pub fn localize<T: serde::Deserialize>(&self) -> LocalizeResult<T> {
        self.localize_data_raw(data::Data::Null)
    }

    /// Same as `localize`, but you provide environment Data for the L20n
    /// files to use.
    pub fn localize_data<
        T: serde::Deserialize,
        D: serde::Serialize
        >(&self, data: D) -> LocalizeResult<T> {
        let mut enc = data::Encoder::new();
        match data.serialize(&mut enc) {
            Err(e) => return Err(EncodeError(e)),
            _ => {}
        }
        self.localize_data_raw(enc.data().unwrap())
    }

    fn localize_data_raw<T: serde::Deserialize>(&self, data: data::Data) -> LocalizeResult<T> {
        let mut map = HashMap::new();
        let ctx = ResolveContext::new(&self.resources, &data);
        for (id, entry) in &self.resources {
            // Only publish public entries. Entries that start with an underscore are helpers.
            if !id.starts_with('_') {
                match entry {
                    &parser::Entity(..) => {
                        map.insert(id.clone(), match entry.resolve_data(&ctx) {
                            Ok(d) => d,
                            Err(e) => return Err(ResolveError(e))
                        });
                    }
                    _ => () // dont localize comments or macros
                }
            }
        }
        let mut dec = data::Decoder::new(data::Data::Map(map));
        match serde::Deserialize::deserialize(&mut dec) {
            Err(e) => Err(DecodeError(e)),
            Ok(t) => Ok(t)
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::Locale;

    /* custom serde impls are hard
    use serde;
    struct Translated {
        hi: String,
        factorial: String,
        mail: String,
    }

    struct Values {
        num: i32
    }
    */

    #[test]
    fn test_locale() {
        let mut locale = Locale::new();
        let src = r#"
        <brand 'Rust' long: 'Rust Lang'>
        <hi 'Hello, {{ brand::long }}!'>
        <many['zero'] { zero: 'none', one: 'one', many: 'too many' }>
        <mail 'Email in your inbox: {{ many.many }}.'>
        <fac($n) { $n == 0 ? 1 : $n * fac($n -1) }>
        <factorial "Factorial of {{ $number }} is {{ fac($number) }}.">
        "#;
        locale.add_resource(src).unwrap();

        let mut data = HashMap::new();
        data.insert("number", 3);

        let t: HashMap<String, String> = locale.localize_data(data).unwrap();

        assert_eq!(t["hi"], "Hello, Rust Lang!");
        assert_eq!(t["factorial"], "Factorial of 3 is 6.");
        assert_eq!(t["mail"], "Email in your inbox: too many.");
    }

}
