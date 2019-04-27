//! Database connection string parsing library for Rust.
//! 
//! This crate contains method to parse and extract part of a connection string in several formats.
//! There is also the possibility to encode a connection string using the `append_key_value` function.
//! 
//! # Supported formats
//! 
//! - Entity Framework (from the .net framework)
//! - MS SQL (from the .net framework System.Data.SqlClient)
//! 
//! # Example
//! 
//! ```
//! use conn_str::{append_key_value, MsSqlConnStr};
//! use std::str::FromStr;
//! 
//! fn main() {
//!     let conn = "data source=.\\SQL2017;initial catalog=Db1;";
//!     let conn = MsSqlConnStr::from_str(conn).unwrap();
//! 
//!     assert_eq!(".\\SQL2017", conn.data_source().unwrap());
//!     assert_eq!("Db1", conn.initial_catalog().unwrap());
//! 
//!     let mut new_conn = String::new();
//! 
//!     append_key_value(&mut new_conn, "data source", conn.data_source().unwrap(), false);
//!     append_key_value(&mut new_conn, "initial catalog", conn.initial_catalog().unwrap(), false);
//!     
//!     // add a user and a password to the connection string
//!     append_key_value(&mut new_conn, "user id", "john", false);
//!     append_key_value(&mut new_conn, "password", "Pass1=3", false);
//! 
//!     assert_eq!(&new_conn, r#"data source=.\SQL2017;initial catalog=Db1;user id=john;password="Pass1=3""#);
//! }
//! ```
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::str::{CharIndices, FromStr};

/// Represent an Entity Framework Connection String
///
/// # Example
///
/// ```
/// use conn_str::EFConnStr;
/// use std::str::FromStr;
///
/// let b = EFConnStr
///     ::from_str(r#"provider=System.Data.SqlClient;provider connection string="server=.\Sql2017;database=Db1""#)
///     .unwrap();
///
/// assert_eq!("System.Data.SqlClient", b.provider().unwrap());
/// assert_eq!("server=.\\Sql2017;database=Db1", b.provider_connection_string().unwrap());
/// ```
pub struct EFConnStr(HashMap<String, String>);

impl FromStr for EFConnStr {
    type Err = Error;

    fn from_str(conn_str: &str) -> Result<Self, Self::Err> {
        Ok(EFConnStr(parse(conn_str, false, None)?))
    }
}

impl EFConnStr {
    pub fn metadata(&self) -> Option<&str> {
        self.0.get("metadata").map(|s| s.as_str())
    }

    pub fn name(&self) -> Option<&str> {
        self.0.get("name").map(|s| s.as_str())
    }

    pub fn provider(&self) -> Option<&str> {
        self.0.get("provider").map(|s| s.as_str())
    }

    pub fn provider_connection_string(&self) -> Option<&str> {
        self.0.get("provider connection string").map(|s| s.as_str())
    }
}

/// Represent a Sql Connection String
///
/// # Example
///
/// ```
/// use conn_str::MsSqlConnStr;
/// use std::str::FromStr;
///
/// let conn = "data source=.\\Sql2017;Initial Catalog=Db1;integrated security=sspi;pwd='Test=1'";
/// let conn = MsSqlConnStr::from_str(conn).unwrap();
///
/// // gets the data_source
/// assert_eq!(".\\Sql2017", conn.data_source().unwrap());
/// 
/// // gets the initial catalog
/// assert_eq!("Db1", conn.initial_catalog().unwrap());
/// ```
pub struct MsSqlConnStr(HashMap<String, String>);

impl FromStr for MsSqlConnStr {
    type Err = Error;

    fn from_str(conn_str: &str) -> Result<Self, Self::Err> {
        Ok(MsSqlConnStr(parse(conn_str, false, None)?))
    }
}

impl MsSqlConnStr {
    pub fn application_name(&self) -> Option<&str> {
        self.0
            .get("application name")
            .or_else(|| self.0.get("app"))
            .map(|s| s.as_str())
    }

    pub fn data_source(&self) -> Option<&str> {
        self.0
            .get("data source")
            .or_else(|| self.0.get("addr"))
            .or_else(|| self.0.get("address"))
            .or_else(|| self.0.get("network address"))
            .or_else(|| self.0.get("server"))
            .map(|s| s.as_str())
    }

    pub fn initial_catalog(&self) -> Option<&str> {
        self.0
            .get("initial catalog")
            .or_else(|| self.0.get("database"))
            .map(|s| s.as_str())
    }

    pub fn integrated_security(&self) -> Result<bool, Error> {
        match self
            .0
            .get("integrated security")
            .or_else(|| self.0.get("trusted_connection"))
        {
            Some(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "sspi" => Ok(true),
                "false" | "no" => Ok(false),
                _ => Err(Error::NotAValidBool(s.to_owned())),
            },
            None => Ok(false),
        }
    }

    pub fn multiple_active_result_sets(&self) -> Result<bool, Error> {
        match self.0.get("multipleactiveresultsets") {
            Some(v) => parse_bool(v),
            None => Ok(false),
        }
    }

    pub fn password(&self) -> Option<&str> {
        self.0
            .get("password")
            .or_else(|| self.0.get("pwd"))
            .map(|s| s.as_str())
    }

    pub fn trust_server_certificate(&self) -> Result<bool, Error> {
        match self.0.get("trustservercertificate") {
            Some(v) => parse_bool(v),
            None => Ok(false),
        }
    }

    pub fn user_id(&self) -> Option<&str> {
        self.0
            .get("user id")
            .or_else(|| self.0.get("uid"))
            .or_else(|| self.0.get("user"))
            .map(|s| s.as_str())
    }
}

/// A Sql Connection String parsing error
#[derive(Clone, Debug)]
pub enum Error {
    KeyNotSupported(String),
    NotAValidBool(String),
    SyntaxError(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::KeyNotSupported(s) => write!(f, "connection string key `{}` not supported", s),
            Error::NotAValidBool(s) => write!(f, "`{}` is not a valid boolean value", s),
            Error::SyntaxError(index) => {
                write!(f, "parsing of connection string failed at `{}`", index)
            }
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::KeyNotSupported(_) => "connection string key not supported",
            Error::NotAValidBool(_) => "not a valid boolean value",
            Error::SyntaxError(_) => "parsing of connection string failed",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

/// Encode a key and value for use in a connection string
///
/// # Example
///
/// ```
/// use conn_str::append_key_value;
///
/// let mut s = String::new();
///
/// append_key_value(&mut s, "database", "MasterDb", false);
/// append_key_value(&mut s, "server", ".\\SQL2017", false);
/// append_key_value(&mut s, "user id", "me", false);
/// append_key_value(&mut s, "password", "pass=1", false);
///
/// assert_eq!("database=MasterDb;server=.\\SQL2017;user id=me;password=\"pass=1\"", &s);
/// ```
pub fn append_key_value(out: &mut String, key: &str, value: &str, use_odbc_rules: bool) {
    if !out.is_empty() && !out.ends_with(';') {
        out.push(';');
    }

    if use_odbc_rules {
        out.push_str(key);
    } else {
        out.push_str(&key.replace("=", "=="));
    }

    out.push('=');

    if use_odbc_rules {
        // should quote the value
        if !value.is_empty()
            && (value.starts_with('{') || value.contains(';') || &value.to_lowercase() == "driver")
            && !quote_odbc_value_match(value)
        {
            out.push('{');
            out.push_str(&value.replace('}', "}}"));
            out.push('}');
        } else {
            out.push_str(value);
        }
    }
    // value already quoted!
    else if quote_value_match(value) {
        out.push_str(value)
    }
    // value contains double quote
    else if value.contains('"') && !value.contains('\'') {
        out.push('\'');
        out.push_str(value);
        out.push('\'');
    }
    // should quote the value
    else {
        out.push('"');
        out.push_str(&value.replace('"', "\"\""));
        out.push('"');
    }
}

#[test]
fn append_key_value_works() {
    let mut out = String::new();
    append_key_value(&mut out, "a", "test=2", false);
    assert_eq!(&out, "a=\"test=2\"");
}

fn parse_bool(s: &str) -> Result<bool, Error> {
    match s.to_lowercase().as_str() {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err(Error::NotAValidBool(s.to_owned())),
    }
}

fn quote_odbc_value_match(s: &str) -> bool {
    // should be identical to the following regex
    // ^{([^}]|}})*}$

    if s.starts_with('{') && s.ends_with('}') {
        let s = &s[1..s.len() - 1];
        !s.contains('}') || s.contains("}}")
    } else {
        false
    }
}

fn quote_value_match(s: &str) -> bool {
    // should be identical to the following regex
    // ^[^\"'=;\\s\\p{Cc}]*$

    !s.chars().any(|c| {
        c == '"' || c == '\'' || c == '=' || c == ';' || c.is_whitespace() || c.is_control()
    })
}

#[test]
fn sql_conn_builder_str_from_str_works() {
    let s = r#"Data Source=.;Initial Catalog=MasterDb;Integrated Security=False;User ID=me;Password="special=321";MultipleActiveResultSets=True;Application Name=RustApp"#;
    let b = MsSqlConnStr::from_str(s).unwrap();

    assert_eq!("special=321", b.password().unwrap());
    assert_eq!("me", b.user_id().unwrap());
}

fn parse(
    conn_str: &str,
    use_odbc_rules: bool,
    synonyms: Option<&HashMap<String, String>>,
) -> Result<HashMap<String, String>, Error> {
    let mut chars = conn_str.char_indices();
    let mut map = HashMap::new();

    while let Some((key, value)) = parse_key_value(&mut chars, use_odbc_rules)? {
        if key.is_empty() {
            break;
        }

        let key = match synonyms {
            Some(synonyms) => match synonyms.get(&key) {
                Some(key) => key.clone(),
                None => return Err(Error::KeyNotSupported(key)),
            },
            None => key,
        };

        if key
            .chars()
            .next()
            .map(|c| c.is_whitespace() || c == ';')
            .unwrap_or(true)
            || key.contains('\0')
        {
            return Err(Error::KeyNotSupported(key));
        }

        map.entry(key).or_insert(value);
    }

    Ok(map)
}

fn parse_key_value(
    chars: &mut CharIndices,
    use_odbc_rules: bool,
) -> Result<Option<(String, String)>, Error> {
    let mut state = State::NothingYet;
    let mut buf = String::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut i = None;

    'next: while let Some((index, c)) = chars.next() {
        i = Some(index);

        // this loop is used to simulate a fallback between state
        // ex: In State::KeyEqual, we have a fallback into State::KeyEnd.
        loop {
            match state {
                State::NothingYet => {
                    if c == ';' || c.is_whitespace() {
                        continue 'next;
                    } else if c == '\0' {
                        state = State::NullTermination;
                        continue 'next;
                    } else if c.is_control() {
                        return Err(Error::SyntaxError(index));
                    } else if c == '=' {
                        state = State::KeyEqual;
                        continue;
                    } else {
                        state = State::Key;
                        buf.push(c);
                        continue 'next;
                    }
                }
                State::Key => {
                    if c == '=' {
                        state = State::KeyEqual;
                        continue 'next;
                    } else if !c.is_whitespace() && c.is_control() {
                        return Err(Error::SyntaxError(index));
                    } else {
                        buf.push(c);
                        continue 'next;
                    }
                }
                State::KeyEqual => {
                    if !use_odbc_rules && c == '=' {
                        state = State::Key;
                        buf.push(c);
                        continue 'next;
                    } else {
                        key = buf.trim_end().to_lowercase();
                        if key.is_empty() {
                            return Err(Error::SyntaxError(index));
                        }

                        buf.clear();
                        state = State::KeyEnd;
                        continue;
                    }
                }
                State::KeyEnd => {
                    if c.is_whitespace() {
                        continue 'next;
                    }
                    if use_odbc_rules {
                        if c == '{' {
                            state = State::BraceQuoteValue;
                            buf.push(c);
                            continue 'next;
                        }
                    } else {
                        if c == '\'' {
                            state = State::SingleQuoteValue;
                            continue 'next;
                        } else if c == '"' {
                            state = State::DoubleQuoteValue;
                            continue 'next;
                        }
                    }

                    if c == ';' || c == '\0' {
                        break;
                    } else if c.is_control() {
                        return Err(Error::SyntaxError(index));
                    }

                    state = State::UnquotedValue;
                    buf.push(c);
                    continue 'next;
                }
                State::UnquotedValue => {
                    if !c.is_whitespace() && (c.is_control() || c == ';') {
                        break;
                    }
                    buf.push(c);
                    continue 'next;
                }
                State::DoubleQuoteValue => {
                    if c == '"' {
                        state = State::DoubleQuoteValueQuote;
                        continue 'next;
                    } else if c == '\0' {
                        return Err(Error::SyntaxError(index));
                    } else {
                        buf.push(c);
                        continue 'next;
                    }
                }
                State::DoubleQuoteValueQuote => {
                    if c == '"' {
                        state = State::DoubleQuoteValue;
                        buf.push(c);
                        continue 'next;
                    } else {
                        value = buf.clone();
                        state = State::QuotedValueEnd;
                        continue;
                    }
                }
                State::SingleQuoteValue => {
                    if c == '\'' {
                        state = State::SingleQuoteValueQuote;
                        continue 'next;
                    } else if c == '\0' {
                        return Err(Error::SyntaxError(index));
                    } else {
                        buf.push(c);
                        continue 'next;
                    }
                }
                State::SingleQuoteValueQuote => {
                    if c == '\'' {
                        state = State::SingleQuoteValue;
                        buf.push(c);
                        continue 'next;
                    } else {
                        value = buf.clone();
                        state = State::QuotedValueEnd;
                        continue;
                    }
                }
                State::BraceQuoteValue => {
                    if c == '}' {
                        state = State::BraceQuoteValueQuote;
                    } else if c == '\0' {
                        return Err(Error::SyntaxError(index));
                    }
                    buf.push(c);
                    continue 'next;
                }
                State::BraceQuoteValueQuote => {
                    if c == '}' {
                        state = State::BraceQuoteValue;
                        buf.push(c);
                        continue 'next;
                    } else {
                        value = buf.clone();
                        state = State::QuotedValueEnd;
                        continue;
                    }
                }
                State::QuotedValueEnd => {
                    if c.is_whitespace() {
                        continue 'next;
                    } else if c != ';' {
                        if c == '\0' {
                            state = State::NullTermination;
                            continue 'next;
                        } else {
                            return Err(Error::SyntaxError(index));
                        }
                    }
                    break;
                }
                State::NullTermination => {
                    if c == '\0' || c.is_whitespace() {
                        continue 'next;
                    }
                    return Err(Error::SyntaxError(index));
                }
            }
        }
        break;
    }

    if let Some(index) = i {
        match state {
            State::Key
            | State::DoubleQuoteValue
            | State::SingleQuoteValue
            | State::BraceQuoteValue => {
                return Err(Error::SyntaxError(index));
            }
            State::KeyEqual => {
                key = buf.trim_end().to_lowercase();
                if buf.is_empty() {
                    return Err(Error::SyntaxError(index));
                }
            }
            State::UnquotedValue => {
                value = buf.trim().to_owned();
                if !use_odbc_rules && (value.ends_with('\'') || value.ends_with('"')) {
                    return Err(Error::SyntaxError(index));
                }
            }
            State::DoubleQuoteValueQuote
            | State::SingleQuoteValueQuote
            | State::BraceQuoteValueQuote
            | State::QuotedValueEnd => {
                value = buf.clone();
            }
            State::NothingYet | State::KeyEnd | State::NullTermination => {}
        }

        Ok(Some((key, value)))
    } else {
        Ok(None)
    }
}

enum State {
    NothingYet,
    Key,
    KeyEqual,
    KeyEnd,
    UnquotedValue,
    DoubleQuoteValue,
    DoubleQuoteValueQuote,
    SingleQuoteValue,
    SingleQuoteValueQuote,
    BraceQuoteValue,
    BraceQuoteValueQuote,
    QuotedValueEnd,
    NullTermination,
}
