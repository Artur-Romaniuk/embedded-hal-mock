//! I²C mock implementations.
//!
//! ## Usage
//!
//! ```
//! extern crate embedded_hal;
//! extern crate embedded_hal_mock;
//!
//! use embedded_hal::i2c::ErrorKind;
//! use embedded_hal::i2c::blocking::I2c;
//! use embedded_hal::i2c::blocking::Operation;
//! use embedded_hal_mock::i2c::{Mock as I2cMock, Transaction as I2cTransaction};
//!
//! // Configure expectations
//! let expectations = [
//!     I2cTransaction::write(0xaa, vec![1, 2]),
//!     I2cTransaction::read(0xbb, vec![3, 4]),
//! ];
//! let mut i2c = I2cMock::new(&expectations);
//!
//! // Writing
//! i2c.write(0xaa, &vec![1, 2]).unwrap();
//!
//! // Reading
//! let mut buf = vec![0u8; 2];
//! i2c.read(0xbb, &mut buf).unwrap();
//! assert_eq!(buf, vec![3, 4]);
//!
//! // Finalise expectations
//! i2c.done();
//! ```
//!
//! ## Transactions
//!
//! There are currently three transaction types:
//!
//! - `Read`: This expects an I²C `read` command and will return the wrapped bytes.
//! - `Write`: This expects an I²C `write` command with the wrapped bytes.
//! - `WriteRead`: This expects an I²C `write_read` command where the
//!   `expected` bytes are written and the `response` bytes are returned.
//!
//! ## Testing Error Handling
//!
//! If you want to test error handling of your code, you can attach an error to
//! a transaction. When the transaction is executed, an error is returned.
//!
//! ```
//! # extern crate embedded_hal;
//! # extern crate embedded_hal_mock;
//! # use embedded_hal::i2c::blocking::I2c;
//! # use embedded_hal::i2c::ErrorKind;
//! # use embedded_hal_mock::i2c::{Mock as I2cMock, Transaction as I2cTransaction};
//!
//! // Configure expectations
//! let expectations = [
//!     I2cTransaction::write(0xaa, vec![1, 2]),
//!     I2cTransaction::read(0xbb, vec![3, 4]).with_error(ErrorKind::Other),
//! ];
//! let mut i2c = I2cMock::new(&expectations);
//!
//! // Writing returns without an error
//! i2c.write(0xaa, &vec![1, 2]).unwrap();
//!
//! // Reading returns an error
//! let mut buf = vec![0u8; 2];
//! let err = i2c.read(0xbb, &mut buf).unwrap_err();
//! assert_eq!(err, ErrorKind::Other);
//!
//! // Finalise expectations
//! i2c.done();
//! ```

use embedded_hal::i2c::blocking as i2c;
use embedded_hal::i2c::ErrorKind;
use embedded_hal::i2c::ErrorType;

use crate::common::Generic;

/// I2C Transaction modes
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// Write transaction
    Write,
    /// Read transaction
    Read,
    /// Write and read transaction
    WriteRead,
    /// Mark the start of a transaction
    TransactionStart,
    /// Mark the end of a transaction
    TransactionEnd,
}

/// I2C Transaction type
///
/// Models an I2C read or write
#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    expected_mode: Mode,
    expected_addr: u8,
    expected_data: Vec<u8>,
    response_data: Vec<u8>,
    /// An optional error return for a transaction.
    ///
    /// This is in addition to the mode to allow validation that the
    /// transaction mode is correct prior to returning the error.
    expected_err: Option<ErrorKind>,
}

impl Transaction {
    /// Create a Write transaction
    pub fn write(addr: u8, expected: Vec<u8>) -> Transaction {
        Transaction {
            expected_mode: Mode::Write,
            expected_addr: addr,
            expected_data: expected,
            response_data: Vec::new(),
            expected_err: None,
        }
    }

    /// Create a Read transaction
    pub fn read(addr: u8, response: Vec<u8>) -> Transaction {
        Transaction {
            expected_mode: Mode::Read,
            expected_addr: addr,
            expected_data: Vec::new(),
            response_data: response,
            expected_err: None,
        }
    }

    /// Create a WriteRead transaction
    pub fn write_read(addr: u8, expected: Vec<u8>, response: Vec<u8>) -> Transaction {
        Transaction {
            expected_mode: Mode::WriteRead,
            expected_addr: addr,
            expected_data: expected,
            response_data: response,
            expected_err: None,
        }
    }

    /// Create nested transactions
    pub fn transaction_start(addr: u8) -> Transaction {
        Transaction {
            expected_mode: Mode::TransactionStart,
            expected_addr: addr,
            expected_data: Vec::new(),
            response_data: Vec::new(),
            expected_err: None,
        }
    }

    /// Create nested transactions
    pub fn transaction_end(addr: u8) -> Transaction {
        Transaction {
            expected_mode: Mode::TransactionEnd,
            expected_addr: addr,
            expected_data: Vec::new(),
            response_data: Vec::new(),
            expected_err: None,
        }
    }

    /// Add an error return to a transaction
    ///
    /// This is used to mock failure behaviours.
    ///
    /// Note: When attaching this to a read transaction, the response in the
    /// expectation will not actually be written to the buffer.
    pub fn with_error(mut self, error: ErrorKind) -> Self {
        self.expected_err = Some(error);
        self
    }
}

/// Mock I2C implementation
///
/// This supports the specification and evaluation of expectations to allow automated testing of I2C based drivers.
/// Mismatches between expectations will cause runtime assertions to assist in locating the source of the fault.
pub type Mock = Generic<Transaction>;

impl ErrorType for Mock {
    type Error = ErrorKind;
}

impl i2c::I2c for Mock {
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let w = self
            .next()
            .expect("no pending expectation for i2c::read call");

        assert_eq!(w.expected_mode, Mode::Read, "i2c::read unexpected mode");
        assert_eq!(w.expected_addr, address, "i2c::read address mismatch");

        assert_eq!(
            buffer.len(),
            w.response_data.len(),
            "i2c:read mismatched response length"
        );

        match w.expected_err {
            Some(err) => Err(err),
            None => {
                buffer.copy_from_slice(&w.response_data);
                Ok(())
            }
        }
    }

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let w = self
            .next()
            .expect("no pending expectation for i2c::write call");

        assert_eq!(w.expected_mode, Mode::Write, "i2c::write unexpected mode");
        assert_eq!(w.expected_addr, address, "i2c::write address mismatch");
        assert_eq!(
            w.expected_data, bytes,
            "i2c::write data does not match expectation"
        );

        match w.expected_err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    fn write_iter<B>(&mut self, address: u8, bytes: B) -> Result<(), Self::Error>
    where
        B: IntoIterator<Item = u8>,
    {
        let w = self
            .next()
            .expect("no pending expectation for i2c::write call");

        assert_eq!(
            w.expected_mode,
            Mode::Write,
            "i2c::write_iter unexpected mode"
        );
        assert_eq!(w.expected_addr, address, "i2c::write_iter address mismatch");
        assert!(
            bytes.into_iter().eq(w.expected_data),
            "i2c::write_iter data does not match expectation"
        );

        match w.expected_err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        let w = self
            .next()
            .expect("no pending expectation for i2c::write_read call");

        assert_eq!(
            w.expected_mode,
            Mode::WriteRead,
            "i2c::write_read unexpected mode"
        );
        assert_eq!(w.expected_addr, address, "i2c::write_read address mismatch");
        assert_eq!(
            w.expected_data, bytes,
            "i2c::write_read write data does not match expectation"
        );

        assert_eq!(
            buffer.len(),
            w.response_data.len(),
            "i2c::write_read mismatched response length"
        );

        match w.expected_err {
            Some(err) => Err(err),
            None => {
                buffer.copy_from_slice(&w.response_data);
                Ok(())
            }
        }
    }

    fn write_iter_read<B>(
        &mut self,
        address: u8,
        bytes: B,
        buffer: &mut [u8],
    ) -> Result<(), Self::Error>
    where
        B: IntoIterator<Item = u8>,
    {
        let w = self
            .next()
            .expect("no pending expectation for i2c::write_read call");

        assert_eq!(
            w.expected_mode,
            Mode::WriteRead,
            "i2c::write_iter_read unexpected mode"
        );
        assert_eq!(
            w.expected_addr, address,
            "i2c::write_iter_read address mismatch"
        );
        assert!(
            bytes.into_iter().eq(w.expected_data),
            "i2c::write_iter_read write data does not match expectation"
        );

        assert_eq!(
            buffer.len(),
            w.response_data.len(),
            "i2c::write_iter_read mismatched response length"
        );

        match w.expected_err {
            Some(err) => Err(err),
            None => {
                buffer.copy_from_slice(&w.response_data);
                Ok(())
            }
        }
    }

    fn transaction<'a>(
        &mut self,
        address: u8,
        operations: &mut [i2c::Operation<'a>],
    ) -> Result<(), Self::Error> {
        let w = self
            .next()
            .expect("no pending expectation for i2c::transaction call");

        assert_eq!(
            w.expected_mode,
            Mode::TransactionStart,
            "i2c::transaction_start unexpected mode"
        );

        for op in operations {
            match op {
                i2c::Operation::Read(r) => self.read(address, r),
                i2c::Operation::Write(w) => self.write(address, w),
            }
            .unwrap();
        }

        let w = self
            .next()
            .expect("no pending expectation for i2c::transaction call");

        assert_eq!(
            w.expected_mode,
            Mode::TransactionEnd,
            "i2c::transaction_end unexpected mode"
        );

        Ok(())
    }

    fn transaction_iter<'a, O>(&mut self, address: u8, operations: O) -> Result<(), Self::Error>
    where
        O: IntoIterator<Item = i2c::Operation<'a>>,
    {
        let w = self
            .next()
            .expect("no pending expectation for i2c::transaction call");

        assert_eq!(
            w.expected_mode,
            Mode::TransactionStart,
            "i2c::transaction_start unexpected mode"
        );

        for op in operations {
            match op {
                i2c::Operation::Read(r) => self.read(address, r),
                i2c::Operation::Write(w) => self.write(address, w),
            }
            .unwrap();
        }

        let w = self
            .next()
            .expect("no pending expectation for i2c::transaction call");

        assert_eq!(
            w.expected_mode,
            Mode::TransactionEnd,
            "i2c::transaction_end unexpected mode"
        );

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use embedded_hal::i2c::blocking::I2c;

    #[test]
    fn test_i2c_mock_write() {
        let expectations = [Transaction::write(0xaa, vec![10, 12])];
        let mut i2c = Mock::new(&expectations);

        i2c.write(0xaa, &vec![10, 12]).unwrap();

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_write_iter() {
        let expectations = [Transaction::write(0xaa, vec![10, 12])];
        let mut i2c = Mock::new(&expectations);

        i2c.write_iter(0xaa, vec![10, 12]).unwrap();

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_read() {
        let expectations = [Transaction::read(0xaa, vec![1, 2])];
        let mut i2c = Mock::new(&expectations);

        let mut buff = vec![0u8; 2];
        i2c.read(0xaa, &mut buff).unwrap();
        assert_eq!(vec![1, 2], buff);

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_write_read() {
        let expectations = [Transaction::write_read(0xaa, vec![1, 2], vec![3, 4])];
        let mut i2c = Mock::new(&expectations);

        let v = vec![1, 2];
        let mut buff = vec![0u8; 2];
        i2c.write_read(0xaa, &v, &mut buff).unwrap();
        assert_eq!(vec![3, 4], buff);

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_write_iter_read() {
        let expectations = [Transaction::write_read(0xaa, vec![1, 2], vec![3, 4])];
        let mut i2c = Mock::new(&expectations);

        let v = vec![1, 2];
        let mut buff = vec![0u8; 2];
        i2c.write_iter_read(0xaa, v, &mut buff).unwrap();
        assert_eq!(vec![3, 4], buff);

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_multiple() {
        let expectations = [
            Transaction::write(0xaa, vec![1, 2]),
            Transaction::read(0xbb, vec![3, 4]),
        ];
        let mut i2c = Mock::new(&expectations);

        i2c.write(0xaa, &vec![1, 2]).unwrap();

        let mut v = vec![0u8; 2];
        i2c.read(0xbb, &mut v).unwrap();

        assert_eq!(v, vec![3, 4]);

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_multiple_transaction() {
        let expectations = [
            Transaction::transaction_start(0xaa),
            Transaction::write(0xaa, vec![1, 2]),
            Transaction::read(0xaa, vec![3, 4]),
            Transaction::transaction_end(0xaa),
        ];
        let mut i2c = Mock::new(&expectations);

        let mut v = vec![0u8; 2];
        i2c.transaction(
            0xaa,
            &mut [
                i2c::Operation::Write(&vec![1, 2]),
                i2c::Operation::Read(&mut v),
            ],
        )
        .unwrap();

        assert_eq!(v, vec![3, 4]);

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_multiple_transaction_iter() {
        let expectations = [
            Transaction::transaction_start(0xaa),
            Transaction::write(0xaa, vec![1, 2]),
            Transaction::read(0xaa, vec![3, 4]),
            Transaction::transaction_end(0xaa),
        ];
        let mut i2c = Mock::new(&expectations);

        let mut v = vec![0u8; 2];
        i2c.transaction_iter(
            0xaa,
            [
                i2c::Operation::Write(&vec![1, 2]),
                i2c::Operation::Read(&mut v),
            ],
        )
        .unwrap();

        assert_eq!(v, vec![3, 4]);

        i2c.done();
    }

    #[test]
    #[should_panic]
    fn test_i2c_mock_write_err() {
        let expectations = [Transaction::write(0xaa, vec![1, 2])];
        let mut i2c = Mock::new(&expectations);

        i2c.write(0xaa, &vec![1, 3]).unwrap();

        i2c.done();
    }

    #[test]
    #[should_panic]
    fn test_i2c_mock_read_err() {
        let expectations = [Transaction::read(0xaa, vec![10, 12])];
        let mut i2c = Mock::new(&expectations);

        let mut buff = vec![0u8; 2];
        i2c.write(0xaa, &mut buff).unwrap();
        assert_eq!(vec![10, 12], buff);

        i2c.done();
    }

    #[test]
    #[should_panic]
    fn test_i2c_mock_write_read_err() {
        let expectations = [Transaction::write_read(0xbb, vec![1, 2], vec![3, 4])];
        let mut i2c = Mock::new(&expectations);

        let v = vec![1, 2];
        let mut buff = vec![0u8; 2];
        i2c.write_read(0xaa, &v, &mut buff).unwrap();
        assert_eq!(vec![3, 4], buff);

        i2c.done();
    }

    #[test]
    #[should_panic]
    fn test_i2c_mock_mode_err() {
        let expectations = [Transaction::read(0xaa, vec![10, 12])];
        let mut i2c = Mock::new(&expectations);

        i2c.write(0xaa, &vec![10, 12]).unwrap();

        i2c.done();
    }

    #[test]
    #[should_panic]
    fn test_i2c_mock_multiple_transaction_err() {
        let expectations = [
            Transaction::write(0xaa, vec![10, 12]),
            Transaction::write(0xaa, vec![10, 12]),
        ];
        let mut i2c = Mock::new(&expectations);

        i2c.write(0xaa, &vec![10, 12]).unwrap();

        i2c.done();
    }

    #[test]
    fn test_i2c_mock_cloned_done_ok() {
        let expectations = [Transaction::read(0xaa, vec![1, 2])];
        let mut i2c = Mock::new(&expectations);
        let mut i2c_clone = i2c.clone();

        let mut buff = vec![0u8; 2];
        i2c.read(0xaa, &mut buff).unwrap();
        assert_eq!(vec![1, 2], buff);

        i2c.done();
        i2c_clone.done();
    }

    #[test]
    #[should_panic]
    fn test_i2c_mock_cloned_done_error() {
        let expectations = [Transaction::read(0xaa, vec![1, 2])];
        let i2c = Mock::new(&expectations);
        let mut i2c_clone = i2c.clone();
        i2c_clone.done();
    }

    mod expect_errors {
        use super::*;

        #[test]
        fn write() {
            let expected_err = ErrorKind::Other;
            let mut i2c = Mock::new(&[
                Transaction::write(0xaa, vec![10, 12]).with_error(expected_err.clone())
            ]);
            let err = i2c.write(0xaa, &vec![10, 12]).unwrap_err();
            assert_eq!(err, expected_err);
            i2c.done();
        }

        /// The transaction mode should still be validated.
        #[test]
        #[should_panic]
        fn write_wrong_mode() {
            let mut i2c =
                Mock::new(&[Transaction::write(0xaa, vec![10, 12]).with_error(ErrorKind::Other)]);
            let mut buf = vec![0u8; 2];
            let _ = i2c.read(0xaa, &mut buf);
        }

        /// The transaction bytes should still be validated.
        #[test]
        #[should_panic]
        fn write_wrong_data() {
            let mut i2c =
                Mock::new(&[Transaction::write(0xaa, vec![10, 12]).with_error(ErrorKind::Other)]);
            let _ = i2c.write(0xaa, &vec![10, 13]);
        }

        #[test]
        fn read() {
            let expected_err = ErrorKind::Other;
            let mut i2c =
                Mock::new(
                    &[Transaction::read(0xaa, vec![10, 12]).with_error(expected_err.clone())],
                );
            let mut buf = vec![0u8; 2];
            let err = i2c.read(0xaa, &mut buf).unwrap_err();
            assert_eq!(err, expected_err);
            i2c.done();
        }

        /// The transaction mode should still be validated.
        #[test]
        #[should_panic]
        fn read_wrong_mode() {
            let mut i2c =
                Mock::new(&[Transaction::read(0xaa, vec![10, 12]).with_error(ErrorKind::Other)]);
            let _ = i2c.write(0xaa, &vec![10, 12]);
        }

        #[test]
        fn write_read() {
            let expected_err = ErrorKind::Other;
            let mut i2c = Mock::new(&[Transaction::write_read(0xaa, vec![10, 12], vec![13, 14])
                .with_error(expected_err.clone())]);
            let mut buf = vec![0u8; 2];
            let err = i2c.write_read(0xaa, &[10, 12], &mut buf).unwrap_err();
            assert_eq!(err, expected_err);
            i2c.done();
        }

        /// The transaction mode should still be validated.
        #[test]
        #[should_panic]
        fn write_read_wrong_mode() {
            let mut i2c = Mock::new(&[Transaction::write_read(0xaa, vec![10, 12], vec![13, 14])
                .with_error(ErrorKind::Other)]);
            let _ = i2c.write(0xaa, &vec![10, 12]);
        }

        /// The transaction bytes should still be validated.
        #[test]
        #[should_panic]
        fn write_read_wrong_data() {
            let mut i2c = Mock::new(&[Transaction::write_read(0xaa, vec![10, 12], vec![13, 14])
                .with_error(ErrorKind::Other)]);
            let mut buf = vec![0u8; 2];
            let _ = i2c.write_read(0xaa, &vec![10, 13], &mut buf);
        }
    }
}
