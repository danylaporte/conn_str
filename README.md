[![Build Status](https://travis-ci.org/danylaporte/conn_str.svg?branch=master)](https://travis-ci.org/danylaporte/conn_str)

Database connection string parsing library for Rust.

This crate contains method to parse and extract part of a connection string in several formats.
There is also the possibility to encode a connection string using the `append_key_value` function.

## Documentation
[API Documentation](https://danylaporte.github.io/conn_str/conn_str)

### Supported formats

- Entity Framework (from the .net framework)
- MS SQL (from the .net framework System.Data.SqlClient)

## Example

```rust
use conn_str::{append_key_value, MsSqlConnStr};
use std::str::FromStr;

fn main() {
    let conn = "data source=.\\SQL2017;initial catalog=Db1;";
    let conn = MsSqlConnStr::from_str(conn).unwrap();

    assert_eq!(".\\SQL2017", conn.data_source().unwrap());
    assert_eq!("Db1", conn.initial_catalog().unwrap());

    let mut new_conn = String::new();

    append_key_value(&mut new_conn, "data source", conn.data_source().unwrap(), false);
    append_key_value(&mut new_conn, "initial catalog", conn.initial_catalog().unwrap(), false);
    
    // add a user and a password to the connection string
    append_key_value(&mut new_conn, "user id", "john", false);
    append_key_value(&mut new_conn, "password", "Pass1=3", false);

    assert_eq!(&new_conn, r#"data source=.\SQL2017;initial catalog=Db1;user id=john;password="Pass1=3""#);
}
```

## License

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0
[http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0) or the MIT license
[http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT), at your
option. This file may not be copied, modified, or distributed
except according to those terms.