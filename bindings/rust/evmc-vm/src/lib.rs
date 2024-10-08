// EVMC: Ethereum Client-VM Connector API.
// Copyright 2019 The EVMC Authors.
// Licensed under the Apache License, Version 2.0.

//! Rust bindings for EVMC (Ethereum Client-VM Connector API).
//!
//! Have a look at evmc-declare to declare an EVMC compatible VM.
//! This crate documents how to use certain data types.

#![allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]

mod container;
mod types;

use std::slice;

pub use container::{EvmcContainer, SteppableEvmcContainer};
pub use evmc_sys as ffi;
pub use types::*;

/// Trait EVMC VMs have to implement.
pub trait EvmcVm {
    /// This is called once at initialisation time.
    fn init() -> Self;

    /// This is called for each supplied option.
    fn set_option(&mut self, _: &str, _: &str) -> Result<(), SetOptionError> {
        Ok(())
    }
    /// This is called for every incoming message.
    fn execute<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
    ) -> ExecutionResult;
}

pub trait SteppableEvmcVm {
    fn step_n<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
        status: StepStatusCode,
        pc: u64,
        gas_refunds: i64,
        stack: &'a mut [Uint256],
        memory: &'a mut [u8],
        last_call_result_data: &'a mut [u8],
        steps: i32,
    ) -> StepResult;
}

/// Error codes for set_option.
#[derive(Debug)]
pub enum SetOptionError {
    InvalidKey,
    InvalidValue,
}

/// EVMC result structure.
#[derive(Debug)]
pub struct ExecutionResult {
    pub status_code: StatusCode,
    pub gas_left: i64,
    pub gas_refund: i64,
    pub output: Option<Box<[u8]>>,
    pub create_address: Option<Address>,
}

#[derive(Debug)]
pub struct StepResult {
    pub step_status_code: StepStatusCode,
    pub status_code: StatusCode,
    pub revision: Revision,
    pub pc: u64,
    pub gas_left: i64,
    pub gas_refund: i64,
    pub output: Option<Box<[u8]>>,
    pub stack: Vec<Uint256>,
    pub memory: Vec<u8>,
    pub last_call_return_data: Option<Vec<u8>>,
}

/// EVMC execution message structure.
#[derive(Debug)]
pub struct ExecutionMessage<'a> {
    kind: MessageKind,
    flags: u32,
    depth: i32,
    gas: i64,
    recipient: Address,
    sender: Address,
    input: Option<&'a [u8]>,
    value: Uint256,
    create2_salt: Bytes32,
    code_address: Address,
    code: Option<&'a [u8]>,
    code_hash: Option<Uint256>,
}

/// EVMC transaction context structure.
#[derive(Debug, Copy, Clone, Hash, PartialEq)]
pub struct ExecutionTxContext<'a> {
    #[doc = "The transaction gas price."]
    pub tx_gas_price: Bytes32,
    #[doc = "The transaction origin account."]
    pub tx_origin: Address,
    #[doc = "The miner of the block."]
    pub block_coinbase: Address,
    #[doc = "The block number."]
    pub block_number: i64,
    #[doc = "The block timestamp."]
    pub block_timestamp: i64,
    #[doc = "The block gas limit."]
    pub block_gas_limit: i64,
    #[doc = "The block previous RANDAO (EIP-4399)."]
    pub block_prev_randao: Bytes32,
    #[doc = "The blockchain's ChainID."]
    pub chain_id: Bytes32,
    #[doc = "The block base fee per gas (EIP-1559, EIP-3198)."]
    pub block_base_fee: Bytes32,
    #[doc = "The blob base fee (EIP-7516)."]
    pub blob_base_fee: Bytes32,
    #[doc = "The array of blob hashes (EIP-4844)."]
    pub blob_hashes: &'a [Bytes32],
    #[doc = "The array of transaction initcodes (TXCREATE)."]
    pub initcodes: &'a [ffi::evmc_tx_initcode],
}

/// EVMC context structure. Exposes the EVMC host functions, message data, and transaction context
/// to the executing VM.
pub struct ExecutionContext<'a> {
    host: &'a ffi::evmc_host_interface,
    context: *mut ffi::evmc_host_context,
    tx_context: ExecutionTxContext<'a>,
}

impl ExecutionResult {
    /// Manually create a result.
    pub fn new(
        _status_code: StatusCode,
        _gas_left: i64,
        _gas_refund: i64,
        _output: Option<Box<[u8]>>,
    ) -> Self {
        ExecutionResult {
            status_code: _status_code,
            gas_left: _gas_left,
            gas_refund: _gas_refund,
            output: _output,
            create_address: None,
        }
    }

    /// Create failure result.
    pub fn failure() -> Self {
        ExecutionResult::new(StatusCode::EVMC_FAILURE, 0, 0, None)
    }

    /// Create a revert result.
    pub fn revert(_gas_left: i64, _output: Option<Box<[u8]>>) -> Self {
        ExecutionResult::new(StatusCode::EVMC_REVERT, _gas_left, 0, _output)
    }

    /// Create a successful result.
    pub fn success(_gas_left: i64, _gas_refund: i64, _output: Option<Box<[u8]>>) -> Self {
        ExecutionResult::new(StatusCode::EVMC_SUCCESS, _gas_left, _gas_refund, _output)
    }

    /// Read the status code.
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// Read the amount of gas left.
    pub fn gas_left(&self) -> i64 {
        self.gas_left
    }

    /// Read the amount of gas refunded.
    pub fn gas_refund(&self) -> i64 {
        self.gas_refund
    }

    /// Read the output returned.
    pub fn output(&self) -> Option<&[u8]> {
        self.output.as_deref()
    }

    /// Read the address of the created account. This will likely be set when
    /// returned from a CREATE/CREATE2.
    pub fn create_address(&self) -> Option<&Address> {
        self.create_address.as_ref()
    }
}

impl StepResult {
    pub fn new(
        step_status_code: StepStatusCode,
        status_code: StatusCode,
        revision: Revision,
        pc: u64,
        gas_left: i64,
        gas_refund: i64,
        output: Option<Box<[u8]>>,
        stack: Vec<Uint256>,
        memory: Vec<u8>,
        last_call_return_data: Option<Vec<u8>>,
    ) -> Self {
        Self {
            step_status_code,
            status_code,
            revision,
            pc,
            gas_left,
            gas_refund,
            output,
            stack,
            memory,
            last_call_return_data,
        }
    }

    pub fn step_status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn pc(&self) -> u64 {
        self.pc
    }

    pub fn gas_left(&self) -> i64 {
        self.gas_left
    }

    pub fn gas_refund(&self) -> i64 {
        self.gas_refund
    }

    pub fn output(&self) -> Option<&[u8]> {
        self.output.as_ref().map(AsRef::as_ref)
    }

    pub fn stack(&self) -> &[Uint256] {
        self.stack.as_ref()
    }

    pub fn memory(&self) -> &[u8] {
        self.memory.as_ref()
    }

    pub fn last_call_return_data(&self) -> Option<&[u8]> {
        self.last_call_return_data.as_ref().map(AsRef::as_ref)
    }
}

impl<'a> ExecutionMessage<'a> {
    pub fn new(
        kind: MessageKind,
        flags: u32,
        depth: i32,
        gas: i64,
        recipient: Address,
        sender: Address,
        input: Option<&'a [u8]>,
        value: Uint256,
        create2_salt: Bytes32,
        code_address: Address,
        code: Option<&'a [u8]>,
        code_hash: Option<Uint256>,
    ) -> Self {
        ExecutionMessage {
            kind,
            flags,
            depth,
            gas,
            recipient,
            sender,
            input,
            value,
            create2_salt,
            code_address,
            code,
            code_hash,
        }
    }

    /// Read the message kind.
    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    /// Read the message flags.
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Read the call depth.
    pub fn depth(&self) -> i32 {
        self.depth
    }

    /// Read the gas limit supplied with the message.
    pub fn gas(&self) -> i64 {
        self.gas
    }

    /// Read the recipient address of the message.
    pub fn recipient(&self) -> &Address {
        &self.recipient
    }

    /// Read the sender address of the message.
    pub fn sender(&self) -> &Address {
        &self.sender
    }

    /// Read the optional input message.
    pub fn input(&self) -> Option<&[u8]> {
        self.input
    }

    /// Read the value of the message.
    pub fn value(&self) -> &Uint256 {
        &self.value
    }

    /// Read the salt for CREATE2. Only valid if the message kind is CREATE2.
    pub fn create2_salt(&self) -> &Bytes32 {
        &self.create2_salt
    }

    /// Read the code address of the message.
    pub fn code_address(&self) -> &Address {
        &self.code_address
    }

    /// Read the optional init code.
    pub fn code(&self) -> Option<&[u8]> {
        self.code
    }

    /// Read the optional code hash.
    pub fn code_hash(&self) -> Option<&Bytes32> {
        self.code_hash.as_ref()
    }
}

impl<'a> ExecutionContext<'a> {
    pub fn new(host: &'a ffi::evmc_host_interface, _context: *mut ffi::evmc_host_context) -> Self {
        let _tx_context = unsafe {
            assert!((*host).get_tx_context.is_some());
            (*host).get_tx_context.unwrap()(_context)
        };

        ExecutionContext {
            host,
            context: _context,
            tx_context: _tx_context.into(),
        }
    }

    /// Retrieve the transaction context.
    pub fn get_tx_context(&self) -> &ExecutionTxContext {
        &self.tx_context
    }

    /// Check if an account exists.
    pub fn account_exists(&self, address: &Address) -> bool {
        unsafe {
            assert!((*self.host).account_exists.is_some());
            (*self.host).account_exists.unwrap()(self.context, address as *const Address)
        }
    }

    /// Read from a storage key.
    pub fn get_storage(&self, address: &Address, key: &Bytes32) -> Bytes32 {
        unsafe {
            assert!((*self.host).get_storage.is_some());
            (*self.host).get_storage.unwrap()(
                self.context,
                address as *const Address,
                key as *const Bytes32,
            )
        }
    }

    /// Set value of a storage key.
    pub fn set_storage(
        &mut self,
        address: &Address,
        key: &Bytes32,
        value: &Bytes32,
    ) -> StorageStatus {
        unsafe {
            assert!((*self.host).set_storage.is_some());
            (*self.host).set_storage.unwrap()(
                self.context,
                address as *const Address,
                key as *const Bytes32,
                value as *const Bytes32,
            )
        }
    }

    /// Get balance of an account.
    pub fn get_balance(&self, address: &Address) -> Uint256 {
        unsafe {
            assert!((*self.host).get_balance.is_some());
            (*self.host).get_balance.unwrap()(self.context, address as *const Address)
        }
    }

    /// Get code size of an account.
    pub fn get_code_size(&self, address: &Address) -> usize {
        unsafe {
            assert!((*self.host).get_code_size.is_some());
            (*self.host).get_code_size.unwrap()(self.context, address as *const Address)
        }
    }

    /// Get code hash of an account.
    pub fn get_code_hash(&self, address: &Address) -> Bytes32 {
        unsafe {
            assert!((*self.host).get_code_hash.is_some());
            (*self.host).get_code_hash.unwrap()(self.context, address as *const Address)
        }
    }

    /// Copy code of an account.
    pub fn copy_code(&self, address: &Address, code_offset: usize, buffer: &mut [u8]) -> usize {
        unsafe {
            assert!((*self.host).copy_code.is_some());
            (*self.host).copy_code.unwrap()(
                self.context,
                address as *const Address,
                code_offset,
                // FIXME: ensure that alignment of the array elements is OK
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        }
    }

    /// Self-destruct the current account.
    pub fn selfdestruct(&mut self, address: &Address, beneficiary: &Address) -> bool {
        unsafe {
            assert!((*self.host).selfdestruct.is_some());
            (*self.host).selfdestruct.unwrap()(
                self.context,
                address as *const Address,
                beneficiary as *const Address,
            )
        }
    }

    /// Call to another account.
    pub fn call(&mut self, message: &ExecutionMessage) -> ExecutionResult {
        // There is no need to make any kind of copies here, because the caller
        // won't go out of scope and ensures these pointers remain valid.
        let input = message.input();
        let input_size = if let Some(input) = input {
            input.len()
        } else {
            0
        };
        let input_data = if let Some(input) = input {
            input.as_ptr()
        } else {
            std::ptr::null() as *const u8
        };
        let code = message.code();
        let code_size = if let Some(code) = code { code.len() } else { 0 };
        let code_data = if let Some(code) = code {
            code.as_ptr()
        } else {
            std::ptr::null() as *const u8
        };
        // Cannot use a nice from trait here because that complicates memory management,
        // evmc_message doesn't have a release() method we could abstract it with.
        let message = ffi::evmc_message {
            kind: message.kind(),
            flags: message.flags(),
            depth: message.depth(),
            gas: message.gas(),
            recipient: *message.recipient(),
            sender: *message.sender(),
            input_data,
            input_size,
            value: *message.value(),
            create2_salt: *message.create2_salt(),
            code_address: *message.code_address(),
            code: code_data,
            code_size,
            code_hash: std::ptr::null(),
        };
        unsafe {
            assert!((*self.host).call.is_some());
            (*self.host).call.unwrap()(self.context, &message as *const ffi::evmc_message).into()
        }
    }

    /// Get block hash of an account.
    pub fn get_block_hash(&self, num: i64) -> Bytes32 {
        unsafe {
            assert!((*self.host).get_block_hash.is_some());
            (*self.host).get_block_hash.unwrap()(self.context, num)
        }
    }

    /// Emit a log.
    pub fn emit_log(&mut self, address: &Address, data: &[u8], topics: &[Bytes32]) {
        unsafe {
            assert!((*self.host).emit_log.is_some());
            (*self.host).emit_log.unwrap()(
                self.context,
                address as *const Address,
                // FIXME: ensure that alignment of the array elements is OK
                data.as_ptr(),
                data.len(),
                topics.as_ptr(),
                topics.len(),
            )
        }
    }

    /// Access an account.
    pub fn access_account(&mut self, address: &Address) -> AccessStatus {
        unsafe {
            assert!((*self.host).access_account.is_some());
            (*self.host).access_account.unwrap()(self.context, address as *const Address)
        }
    }

    /// Access a storage key.
    pub fn access_storage(&mut self, address: &Address, key: &Bytes32) -> AccessStatus {
        unsafe {
            assert!((*self.host).access_storage.is_some());
            (*self.host).access_storage.unwrap()(
                self.context,
                address as *const Address,
                key as *const Bytes32,
            )
        }
    }

    /// Read from a transient storage key.
    pub fn get_transient_storage(&self, address: &Address, key: &Bytes32) -> Bytes32 {
        unsafe {
            assert!(self.host.get_transient_storage.is_some());
            self.host.get_transient_storage.unwrap()(
                self.context,
                address as *const Address,
                key as *const Bytes32,
            )
        }
    }

    /// Set value of a transient storage key.
    pub fn set_transient_storage(&mut self, address: &Address, key: &Bytes32, value: &Bytes32) {
        unsafe {
            assert!(self.host.set_transient_storage.is_some());
            self.host.set_transient_storage.unwrap()(
                self.context,
                address as *const Address,
                key as *const Bytes32,
                value as *const Bytes32,
            )
        }
    }
}

impl From<ffi::evmc_result> for ExecutionResult {
    fn from(result: ffi::evmc_result) -> Self {
        let ret = Self {
            status_code: result.status_code,
            gas_left: result.gas_left,
            gas_refund: result.gas_refund,
            output: if result.output_data.is_null() {
                assert_eq!(result.output_size, 0);
                None
            } else if result.output_size == 0 {
                None
            } else {
                Some(Box::from(unsafe {
                    slice::from_raw_parts(result.output_data as *mut u8, result.output_size)
                }))
            },
            // Consider it is always valid.
            create_address: Some(result.create_address),
        };

        // Release allocated ffi struct.
        if let Some(release) = result.release {
            unsafe {
                release(&result);
            }
        }

        ret
    }
}

fn allocate_output_data<T>(output: Option<&Vec<T>>) -> (*const T, usize) {
    if let Some(buf) = output {
        if !buf.is_empty() {
            let buf_len = buf.len();

            // Manually allocate heap memory for the new home of the output buffer.
            let memlayout =
                std::alloc::Layout::from_size_align(buf_len * size_of::<T>(), align_of::<T>())
                    .expect("Bad layout");
            let new_buf = unsafe { std::alloc::alloc(memlayout) as *mut T };
            unsafe {
                // Copy the data into the allocated buffer.
                std::ptr::copy(buf.as_ptr(), new_buf, buf_len);
            }

            return (new_buf as *const T, buf_len);
        }
    }
    (std::ptr::null(), 0)
}

unsafe fn deallocate_output_data<T>(ptr: *const T, size: usize) {
    if !ptr.is_null() {
        let buf_layout =
            std::alloc::Layout::from_size_align(size * size_of::<T>(), align_of::<T>())
                .expect("Bad layout");
        std::alloc::dealloc(ptr as *mut u8, buf_layout);
    }
}

/// Returns a pointer to a heap-allocated evmc_result.
impl From<ExecutionResult> for *const ffi::evmc_result {
    fn from(value: ExecutionResult) -> Self {
        let mut result: ffi::evmc_result = value.into();
        result.release = Some(release_heap_result);
        Box::into_raw(Box::new(result))
    }
}

/// Callback to pass across FFI, de-allocating the optional output_data.
extern "C" fn release_heap_result(result: *const ffi::evmc_result) {
    unsafe {
        let tmp = Box::from_raw(result as *mut ffi::evmc_result);
        deallocate_output_data(tmp.output_data, tmp.output_size);
    }
}

/// Returns a pointer to a stack-allocated evmc_result.
impl From<ExecutionResult> for ffi::evmc_result {
    fn from(value: ExecutionResult) -> Self {
        let (buffer, len) = if let Some(buf) = value.output {
            let len = buf.len();
            (Box::<[u8]>::into_raw(buf) as *const u8, len)
        } else {
            (std::ptr::null(), 0)
        };
        Self {
            status_code: value.status_code,
            gas_left: value.gas_left,
            gas_refund: value.gas_refund,
            output_data: buffer,
            output_size: len,
            release: Some(release_stack_result),
            create_address: if value.create_address.is_some() {
                value.create_address.unwrap()
            } else {
                Address { bytes: [0u8; 20] }
            },
            padding: [0u8; 4],
        }
    }
}

/// Returns a pointer to a stack-allocated evmc_step_result.
impl From<StepResult> for ffi::evmc_step_result {
    fn from(value: StepResult) -> Self {
        let (output_data, output_size) = if let Some(buf) = value.output {
            let len = buf.len();
            (Box::<[u8]>::into_raw(buf) as *const u8, len)
        } else {
            (std::ptr::null(), 0)
        };
        let (stack, stack_size) = allocate_output_data(Some(&value.stack));
        let (memory, memory_size) = allocate_output_data(Some(&value.memory));
        let (last_call_return_data, last_call_return_data_size) =
            allocate_output_data(value.last_call_return_data.as_ref());

        Self {
            step_status_code: value.step_status_code,
            status_code: value.status_code,
            revision: value.revision,
            pc: value.pc,
            gas_left: value.gas_left,
            gas_refund: value.gas_refund,
            output_data,
            output_size,
            stack,
            stack_size,
            memory,
            memory_size,
            last_call_return_data,
            last_call_return_data_size,
            release: Some(release_stack_step_result),
        }
    }
}

/// Converts ffi type into steppable interface result type.
impl From<ffi::evmc_step_result> for StepResult {
    fn from(value: ffi::evmc_step_result) -> Self {
        let ret = Self {
            step_status_code: value.step_status_code,
            status_code: value.status_code,
            revision: value.revision,
            pc: value.pc,
            gas_left: value.gas_left,
            gas_refund: value.gas_refund,
            output: if value.output_data.is_null() {
                assert_eq!(value.output_size, 0);
                None
            } else if value.output_size == 0 {
                None
            } else {
                Some(Box::from(unsafe {
                    slice::from_raw_parts(value.output_data as *mut u8, value.output_size)
                }))
            },
            stack: if value.stack.is_null() {
                assert_eq!(value.stack_size, 0);
                Vec::new()
            } else if value.stack_size == 0 {
                Vec::new()
            } else {
                unsafe {
                    Vec::from(slice::from_raw_parts(
                        value.stack as *mut _,
                        value.stack_size,
                    ))
                }
            },
            memory: if value.memory.is_null() {
                assert_eq!(value.memory_size, 0);
                Vec::new()
            } else if value.memory_size == 0 {
                Vec::new()
            } else {
                unsafe {
                    Vec::from(slice::from_raw_parts(
                        value.memory as *mut _,
                        value.memory_size,
                    ))
                }
            },
            last_call_return_data: if value.last_call_return_data.is_null() {
                assert_eq!(value.last_call_return_data_size, 0);
                None
            } else if value.last_call_return_data_size == 0 {
                None
            } else {
                Some(unsafe {
                    Vec::from(slice::from_raw_parts(
                        value.last_call_return_data as *mut _,
                        value.last_call_return_data_size,
                    ))
                })
            },
        };

        // If release function is provided, use it to release resources.
        if let Some(release) = value.release {
            unsafe {
                release(&value);
            }
        }

        ret
    }
}

/// Callback to pass across FFI, de-allocating the optional output_data.
extern "C" fn release_stack_result(result: *const ffi::evmc_result) {
    unsafe {
        let tmp = *result;
        deallocate_output_data(tmp.output_data, tmp.output_size);
    }
}

/// Callback to pass across FFI, de-allocating all allocated fields.
extern "C" fn release_stack_step_result(result: *const ffi::evmc_step_result) {
    unsafe {
        let tmp = *result;
        deallocate_output_data(tmp.output_data, tmp.output_size);
        deallocate_output_data(tmp.stack, tmp.stack_size);
        deallocate_output_data(tmp.memory, tmp.memory_size);
        deallocate_output_data(tmp.last_call_return_data, tmp.last_call_return_data_size);
    }
}

impl<'a> From<&'a ffi::evmc_message> for ExecutionMessage<'a> {
    fn from(message: &'a ffi::evmc_message) -> Self {
        ExecutionMessage {
            kind: message.kind,
            flags: message.flags,
            depth: message.depth,
            gas: message.gas,
            recipient: message.recipient,
            sender: message.sender,
            input: if message.input_data.is_null() {
                assert_eq!(message.input_size, 0);
                None
            } else if message.input_size == 0 {
                None
            } else {
                Some(unsafe { slice::from_raw_parts(message.input_data, message.input_size) })
            },
            value: message.value,
            create2_salt: message.create2_salt,
            code_address: message.code_address,
            code: if message.code.is_null() {
                assert_eq!(message.code_size, 0);
                None
            } else if message.code_size == 0 {
                None
            } else {
                Some(unsafe { slice::from_raw_parts(message.code, message.code_size) })
            },
            code_hash: if message.code_hash.is_null() {
                None
            } else {
                Some(unsafe { *message.code_hash })
            },
        }
    }
}

impl<'a> From<ffi::evmc_tx_context> for ExecutionTxContext<'a> {
    fn from(message: ffi::evmc_tx_context) -> Self {
        let blob_hashes = if message.blob_hashes.is_null() {
            &[]
        } else {
            assert_ne!(message.blob_hashes_count, 0);
            unsafe { slice::from_raw_parts(message.blob_hashes, message.blob_hashes_count) }
        };
        let initcodes = if message.initcodes.is_null() {
            &[]
        } else {
            assert_ne!(message.initcodes_count, 0);
            unsafe { slice::from_raw_parts(message.initcodes, message.initcodes_count) }
        };
        ExecutionTxContext {
            tx_gas_price: message.tx_gas_price,
            tx_origin: message.tx_origin,
            block_coinbase: message.block_coinbase,
            block_number: message.block_number,
            block_timestamp: message.block_timestamp,
            block_gas_limit: message.block_gas_limit,
            block_prev_randao: message.block_prev_randao,
            chain_id: message.chain_id,
            block_base_fee: message.block_base_fee,
            blob_base_fee: message.blob_base_fee,
            blob_hashes,
            initcodes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_new() {
        let r = ExecutionResult::new(StatusCode::EVMC_FAILURE, 420, 21, None);

        assert_eq!(r.status_code(), StatusCode::EVMC_FAILURE);
        assert_eq!(r.gas_left(), 420);
        assert_eq!(r.gas_refund(), 21);
        assert!(r.output().is_none());
        assert!(r.create_address().is_none());
    }

    // Test-specific helper to dispose of execution results in unit tests
    extern "C" fn test_result_dispose(result: *const ffi::evmc_result) {
        unsafe {
            if !result.is_null() {
                let owned = *result;
                Vec::from_raw_parts(
                    owned.output_data as *mut u8,
                    owned.output_size,
                    owned.output_size,
                );
            }
        }
    }

    #[test]
    fn result_from_ffi() {
        let f = ffi::evmc_result {
            status_code: StatusCode::EVMC_SUCCESS,
            gas_left: 1337,
            gas_refund: 21,
            output_data: Box::into_raw(Box::new([0xde, 0xad, 0xbe, 0xef])) as *const u8,
            output_size: 4,
            release: Some(test_result_dispose),
            create_address: Address { bytes: [0u8; 20] },
            padding: [0u8; 4],
        };

        let r: ExecutionResult = f.into();

        assert_eq!(r.status_code(), StatusCode::EVMC_SUCCESS);
        assert_eq!(r.gas_left(), 1337);
        assert_eq!(r.gas_refund(), 21);
        assert!(r.output().is_some());
        assert_eq!(r.output().unwrap().len(), 4);
        assert!(r.create_address().is_some());
    }

    #[test]
    fn result_into_heap_ffi() {
        let r = ExecutionResult::new(
            StatusCode::EVMC_FAILURE,
            420,
            21,
            Some(Box::new([0xc0, 0xff, 0xee, 0x71, 0x75])),
        );

        let f: *const ffi::evmc_result = r.into();
        assert!(!f.is_null());
        unsafe {
            assert_eq!((*f).status_code, StatusCode::EVMC_FAILURE);
            assert_eq!((*f).gas_left, 420);
            assert_eq!((*f).gas_refund, 21);
            assert!(!(*f).output_data.is_null());
            assert_eq!((*f).output_size, 5);
            assert_eq!(
                std::slice::from_raw_parts((*f).output_data, 5) as &[u8],
                &[0xc0, 0xff, 0xee, 0x71, 0x75]
            );
            assert_eq!((*f).create_address.bytes, [0u8; 20]);
            if (*f).release.is_some() {
                (*f).release.unwrap()(f);
            }
        }
    }

    #[test]
    fn result_into_heap_ffi_empty_data() {
        let r = ExecutionResult::new(StatusCode::EVMC_FAILURE, 420, 21, None);

        let f: *const ffi::evmc_result = r.into();
        assert!(!f.is_null());
        unsafe {
            assert_eq!((*f).status_code, StatusCode::EVMC_FAILURE);
            assert_eq!((*f).gas_left, 420);
            assert_eq!((*f).gas_refund, 21);
            assert!((*f).output_data.is_null());
            assert_eq!((*f).output_size, 0);
            assert_eq!((*f).create_address.bytes, [0u8; 20]);
            if (*f).release.is_some() {
                (*f).release.unwrap()(f);
            }
        }
    }

    #[test]
    fn result_into_stack_ffi() {
        let r = ExecutionResult::new(
            StatusCode::EVMC_FAILURE,
            420,
            21,
            Some(Box::new([0xc0, 0xff, 0xee, 0x71, 0x75])),
        );

        let f: ffi::evmc_result = r.into();
        unsafe {
            assert_eq!(f.status_code, StatusCode::EVMC_FAILURE);
            assert_eq!(f.gas_left, 420);
            assert_eq!(f.gas_refund, 21);
            assert!(!f.output_data.is_null());
            assert_eq!(f.output_size, 5);
            assert_eq!(
                std::slice::from_raw_parts(f.output_data, 5) as &[u8],
                &[0xc0, 0xff, 0xee, 0x71, 0x75]
            );
            assert_eq!(f.create_address.bytes, [0u8; 20]);
            if f.release.is_some() {
                f.release.unwrap()(&f);
            }
        }
    }

    #[test]
    fn result_into_stack_ffi_empty_data() {
        let r = ExecutionResult::new(StatusCode::EVMC_FAILURE, 420, 21, None);

        let f: ffi::evmc_result = r.into();
        unsafe {
            assert_eq!(f.status_code, StatusCode::EVMC_FAILURE);
            assert_eq!(f.gas_left, 420);
            assert_eq!(f.gas_refund, 21);
            assert!(f.output_data.is_null());
            assert_eq!(f.output_size, 0);
            assert_eq!(f.create_address.bytes, [0u8; 20]);
            if f.release.is_some() {
                f.release.unwrap()(&f);
            }
        }
    }

    #[test]
    fn message_new_with_input() {
        let input = vec![0xc0, 0xff, 0xee];
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Bytes32 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };

        let ret = ExecutionMessage::new(
            MessageKind::EVMC_CALL,
            44,
            66,
            4466,
            recipient,
            sender,
            Some(&input),
            value,
            create2_salt,
            code_address,
            None,
            None,
        );

        assert_eq!(ret.kind(), MessageKind::EVMC_CALL);
        assert_eq!(ret.flags(), 44);
        assert_eq!(ret.depth(), 66);
        assert_eq!(ret.gas(), 4466);
        assert_eq!(*ret.recipient(), recipient);
        assert_eq!(*ret.sender(), sender);
        assert!(ret.input().is_some());
        assert_eq!(*ret.input().unwrap(), input);
        assert_eq!(*ret.value(), value);
        assert_eq!(*ret.create2_salt(), create2_salt);
        assert_eq!(*ret.code_address(), code_address);
    }

    #[test]
    fn message_new_with_code() {
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Bytes32 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };
        let code = vec![0x5f, 0x5f, 0xfd];

        let ret = ExecutionMessage::new(
            MessageKind::EVMC_CALL,
            44,
            66,
            4466,
            recipient,
            sender,
            None,
            value,
            create2_salt,
            code_address,
            Some(&code),
            None,
        );

        assert_eq!(ret.kind(), MessageKind::EVMC_CALL);
        assert_eq!(ret.flags(), 44);
        assert_eq!(ret.depth(), 66);
        assert_eq!(ret.gas(), 4466);
        assert_eq!(*ret.recipient(), recipient);
        assert_eq!(*ret.sender(), sender);
        assert_eq!(*ret.value(), value);
        assert_eq!(*ret.create2_salt(), create2_salt);
        assert_eq!(*ret.code_address(), code_address);
        assert!(ret.code().is_some());
        assert_eq!(*ret.code().unwrap(), code);
    }

    #[test]
    fn message_from_ffi() {
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Bytes32 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };

        let msg = ffi::evmc_message {
            kind: MessageKind::EVMC_CALL,
            flags: 44,
            depth: 66,
            gas: 4466,
            recipient,
            sender,
            input_data: std::ptr::null(),
            input_size: 0,
            value,
            create2_salt,
            code_address,
            code: std::ptr::null(),
            code_size: 0,
            code_hash: std::ptr::null(),
        };

        let ret: ExecutionMessage = (&msg).into();

        assert_eq!(ret.kind(), msg.kind);
        assert_eq!(ret.flags(), msg.flags);
        assert_eq!(ret.depth(), msg.depth);
        assert_eq!(ret.gas(), msg.gas);
        assert_eq!(*ret.recipient(), msg.recipient);
        assert_eq!(*ret.sender(), msg.sender);
        assert!(ret.input().is_none());
        assert_eq!(*ret.value(), msg.value);
        assert_eq!(*ret.create2_salt(), msg.create2_salt);
        assert_eq!(*ret.code_address(), msg.code_address);
        assert!(ret.code().is_none());
    }

    #[test]
    fn message_from_ffi_with_input() {
        let input = vec![0xc0, 0xff, 0xee];
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Bytes32 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };

        let msg = ffi::evmc_message {
            kind: MessageKind::EVMC_CALL,
            flags: 44,
            depth: 66,
            gas: 4466,
            recipient,
            sender,
            input_data: input.as_ptr(),
            input_size: input.len(),
            value,
            create2_salt,
            code_address,
            code: std::ptr::null(),
            code_size: 0,
            code_hash: std::ptr::null(),
        };

        let ret: ExecutionMessage = (&msg).into();

        assert_eq!(ret.kind(), msg.kind);
        assert_eq!(ret.flags(), msg.flags);
        assert_eq!(ret.depth(), msg.depth);
        assert_eq!(ret.gas(), msg.gas);
        assert_eq!(*ret.recipient(), msg.recipient);
        assert_eq!(*ret.sender(), msg.sender);
        assert!(ret.input().is_some());
        assert_eq!(*ret.input().unwrap(), input);
        assert_eq!(*ret.value(), msg.value);
        assert_eq!(*ret.create2_salt(), msg.create2_salt);
        assert_eq!(*ret.code_address(), msg.code_address);
        assert!(ret.code().is_none());
    }

    #[test]
    fn message_from_ffi_with_code() {
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Bytes32 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };
        let code = vec![0x5f, 0x5f, 0xfd];

        let msg = ffi::evmc_message {
            kind: MessageKind::EVMC_CALL,
            flags: 44,
            depth: 66,
            gas: 4466,
            recipient,
            sender,
            input_data: std::ptr::null(),
            input_size: 0,
            value,
            create2_salt,
            code_address,
            code: code.as_ptr(),
            code_size: code.len(),
            code_hash: std::ptr::null(),
        };

        let ret: ExecutionMessage = (&msg).into();

        assert_eq!(ret.kind(), msg.kind);
        assert_eq!(ret.flags(), msg.flags);
        assert_eq!(ret.depth(), msg.depth);
        assert_eq!(ret.gas(), msg.gas);
        assert_eq!(*ret.recipient(), msg.recipient);
        assert_eq!(*ret.sender(), msg.sender);
        assert!(ret.input().is_none());
        assert_eq!(*ret.value(), msg.value);
        assert_eq!(*ret.create2_salt(), msg.create2_salt);
        assert_eq!(*ret.code_address(), msg.code_address);
        assert!(ret.code().is_some());
        assert_eq!(*ret.code().unwrap(), code);
    }

    unsafe extern "C" fn get_dummy_tx_context(
        _context: *mut ffi::evmc_host_context,
    ) -> ffi::evmc_tx_context {
        ffi::evmc_tx_context {
            tx_gas_price: Uint256 { bytes: [0u8; 32] },
            tx_origin: Address { bytes: [0u8; 20] },
            block_coinbase: Address { bytes: [0u8; 20] },
            block_number: 42,
            block_timestamp: 235117,
            block_gas_limit: 105023,
            block_prev_randao: Uint256 { bytes: [0xaa; 32] },
            chain_id: Uint256::default(),
            block_base_fee: Uint256::default(),
            blob_base_fee: Uint256::default(),
            blob_hashes: std::ptr::null(),
            blob_hashes_count: 0,
            initcodes: std::ptr::null(),
            initcodes_count: 0,
        }
    }

    unsafe extern "C" fn get_dummy_code_size(
        _context: *mut ffi::evmc_host_context,
        _addr: *const Address,
    ) -> usize {
        105023_usize
    }

    unsafe extern "C" fn execute_call(
        _context: *mut ffi::evmc_host_context,
        _msg: *const ffi::evmc_message,
    ) -> ffi::evmc_result {
        // Some dumb validation for testing.
        let msg = *_msg;
        let success = if msg.input_size != 0 && msg.input_data.is_null() {
            false
        } else {
            msg.input_size != 0 || msg.input_data.is_null()
        };

        ffi::evmc_result {
            status_code: if success {
                StatusCode::EVMC_SUCCESS
            } else {
                StatusCode::EVMC_INTERNAL_ERROR
            },
            gas_left: 2,
            gas_refund: 0,
            // NOTE: we are passing the input pointer here, but for testing the lifetime is ok
            output_data: msg.input_data,
            output_size: msg.input_size,
            release: None,
            create_address: Address::default(),
            padding: [0u8; 4],
        }
    }

    // Update these when needed for tests
    fn get_dummy_host_interface() -> ffi::evmc_host_interface {
        ffi::evmc_host_interface {
            account_exists: None,
            get_storage: None,
            set_storage: None,
            get_balance: None,
            get_code_size: Some(get_dummy_code_size),
            get_code_hash: None,
            copy_code: None,
            selfdestruct: None,
            call: Some(execute_call),
            get_tx_context: Some(get_dummy_tx_context),
            get_block_hash: None,
            emit_log: None,
            access_account: None,
            access_storage: None,
            get_transient_storage: None,
            set_transient_storage: None,
        }
    }

    #[test]
    fn execution_context() {
        let host_context = std::ptr::null_mut();
        let host_interface = get_dummy_host_interface();
        let exe_context = ExecutionContext::new(&host_interface, host_context);
        let a = exe_context.get_tx_context();

        let b = unsafe { get_dummy_tx_context(host_context) };

        assert_eq!(a.block_gas_limit, b.block_gas_limit);
        assert_eq!(a.block_timestamp, b.block_timestamp);
        assert_eq!(a.block_number, b.block_number);
    }

    #[test]
    fn get_code_size() {
        // This address is useless. Just a dummy parameter for the interface function.
        let test_addr = Address { bytes: [0u8; 20] };
        let host = get_dummy_host_interface();
        let host_context = std::ptr::null_mut();

        let exe_context = ExecutionContext::new(&host, host_context);

        let a: usize = 105023;
        let b = exe_context.get_code_size(&test_addr);

        assert_eq!(a, b);
    }

    #[test]
    fn test_call_empty_data() {
        // This address is useless. Just a dummy parameter for the interface function.
        let test_addr = Address::default();
        let host = get_dummy_host_interface();
        let host_context = std::ptr::null_mut();
        let mut exe_context = ExecutionContext::new(&host, host_context);

        let message = ExecutionMessage::new(
            MessageKind::EVMC_CALL,
            0,
            0,
            6566,
            test_addr,
            test_addr,
            None,
            Uint256::default(),
            Bytes32::default(),
            test_addr,
            None,
            None,
        );

        let b = exe_context.call(&message);

        assert_eq!(b.status_code(), StatusCode::EVMC_SUCCESS);
        assert_eq!(b.gas_left(), 2);
        assert!(b.output().is_none());
        assert!(b.create_address().is_some());
        assert_eq!(b.create_address().unwrap(), &Address::default());
    }

    #[test]
    fn test_call_with_data() {
        // This address is useless. Just a dummy parameter for the interface function.
        let test_addr = Address::default();
        let host = get_dummy_host_interface();
        let host_context = std::ptr::null_mut();
        let mut exe_context = ExecutionContext::new(&host, host_context);

        let data = vec![0xc0, 0xff, 0xfe];

        let message = ExecutionMessage::new(
            MessageKind::EVMC_CALL,
            0,
            0,
            6566,
            test_addr,
            test_addr,
            Some(&data),
            Uint256::default(),
            Bytes32::default(),
            test_addr,
            None,
            None,
        );

        let b = exe_context.call(&message);

        assert_eq!(b.status_code(), StatusCode::EVMC_SUCCESS);
        assert_eq!(b.gas_left(), 2);
        assert!(b.output().is_some());
        assert_eq!(b.output().unwrap(), &data);
        assert!(b.create_address().is_some());
        assert_eq!(b.create_address().unwrap(), &Address::default());
    }
}
