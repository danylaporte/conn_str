use std::collections::HashMap;
use std::error;
use std::fmt;
use std::mem::replace;
use std::str::{CharIndices, FromStr};

/// A Sql Connection String parsing error
#[derive(Clone, Debug)]
pub enum Error {
    KeyNotSupported(String),
    SyntaxError(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::KeyNotSupported(s) => {
                write!(f, "sql connection string key `{}` not supported", s)
            }
            Error::SyntaxError(index) => {
                write!(f, "parsing of sql connection string failed at `{}`", index)
            }
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "sql connection string key not supported"
    }

    fn cause(&self) -> Option<&error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

/// Represent an Entity Framework Connection String
///
/// # Example
///
/// ```
/// use conn_str::EntityConnStrBuilder;
/// use std::str::FromStr;
///
/// let b = EntityConnStrBuilder
///     ::from_str(r#"provider=System.Data.SqlClient;provider connection string="server=.\Sql2017;database=Db1""#)
///     .unwrap();
///
/// assert_eq!("System.Data.SqlClient", b.provider().unwrap());
/// assert_eq!("server=.\\Sql2017;database=Db1", b.provider_connection_string().unwrap());
/// ```
pub struct EntityConnStrBuilder(HashMap<String, String>);

impl FromStr for EntityConnStrBuilder {
    type Err = Error;
    fn from_str(conn_str: &str) -> Result<Self, Self::Err> {
        Ok(EntityConnStrBuilder(parse(conn_str, false, None)?))
    }
}

impl EntityConnStrBuilder {
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
/// use conn_str::SqlConnStrBuilder;
/// use std::str::FromStr;
///
/// let b = SqlConnStrBuilder::from_str("data source=.\\Sql2017;Initial Catalog=Db1;integrated security=sspi;pwd='Test=1'").unwrap();
///
/// assert_eq!(".\\Sql2017", b.data_source().unwrap());
/// assert_eq!("Db1", b.initial_catalog().unwrap());
/// assert_eq!(true, b.integrated_security());
/// assert_eq!("Test=1", b.password().unwrap());
/// ```
pub struct SqlConnStrBuilder(HashMap<String, String>);

impl FromStr for SqlConnStrBuilder {
    type Err = Error;

    fn from_str(conn_str: &str) -> Result<Self, Self::Err> {
        Ok(SqlConnStrBuilder(parse(conn_str, false, None)?))
    }
}

impl SqlConnStrBuilder {
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

    pub fn integrated_security(&self) -> bool {
        self.0
            .get("integrated security")
            .or_else(|| self.0.get("trusted_connection"))
            .map(|v| match v.to_lowercase().as_str() {
                "true" | "sspi" | "1" => true,
                _ => false,
            })
            .unwrap_or(false)
    }

    pub fn password(&self) -> Option<&str> {
        self.0
            .get("password")
            .or_else(|| self.0.get("pwd"))
            .map(|s| s.as_str())
    }

    pub fn user_id(&self) -> Option<&str> {
        self.0
            .get("user id")
            .or_else(|| self.0.get("uid"))
            .or_else(|| self.0.get("user"))
            .map(|s| s.as_str())
    }
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
                        value = replace(&mut buf, String::new());
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
                        value = replace(&mut buf, String::new());
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
                        value = replace(&mut buf, String::new());
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
                value = replace(&mut buf, String::new());
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
