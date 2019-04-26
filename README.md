[![Build Status](https://travis-ci.org/danylaporte/conn_str.svg?branch=master)](https://travis-ci.org/danylaporte/conn_str)

A connection string parsing lib for rust.

## Documentation
[API Documentation](https://danylaporte.github.io/conn_str/conn_str)

## Example

```rust
/// use conn_str::SqlConnStrBuilder;
/// use std::str::FromStr;
///
/// let b = SqlConnStrBuilder::from_str("data source=.\\Sql2017;Initial Catalog=Db1;integrated security=sspi;pwd='Test=1'").unwrap();
///
/// assert_eq!(".\\Sql2017", b.data_source().unwrap());
/// assert_eq!("Db1", b.initial_catalog().unwrap());
/// assert_eq!(true, b.integrated_security());
/// assert_eq!("Test=1", b.password().unwrap());
```

## License

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0
[http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0) or the MIT license
[http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT), at your
option. This file may not be copied, modified, or distributed
except according to those terms.