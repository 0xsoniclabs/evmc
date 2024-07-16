// EVMC: Ethereum Client-VM Connector API.
// Copyright 2018 The EVMC Authors.
// Licensed under the Apache License, Version 2.0.

package evmc

/*
#cgo CFLAGS: -I${SRCDIR}/../../../include -Wall -Wextra
#cgo !windows LDFLAGS: -ldl

#include <evmc/evmc.h>
#include <evmc/helpers.h>
#include <evmc/loader.h>

#include <stdlib.h>
#include <string.h>

static inline enum evmc_set_option_result set_option(struct evmc_vm* vm, char* name, char* value)
{
	enum evmc_set_option_result ret = evmc_set_option(vm, name, value);
	free(name);
	free(value);
	return ret;
}

extern const struct evmc_host_interface evmc_go_host;

static struct evmc_result execute_wrapper(struct evmc_vm* vm,
	uintptr_t context_index, enum evmc_revision rev,
	enum evmc_call_kind kind, uint32_t flags, int32_t depth, int64_t gas,
	const evmc_address* recipient, const evmc_address* sender,
	const uint8_t* input_data, size_t input_size, const evmc_uint256be* value,
	const evmc_bytes32* code_hash, const uint8_t* code, size_t code_size)
{
	struct evmc_message msg = {
		kind,
		flags,
		depth,
		gas,
		*recipient,
		*sender,
		input_data,
		input_size,
		*value,
		{{0}}, // create2_salt: not required for execution
		{{0}}, // code_address: not required for execution
		0,     // code
		0,     // code_size
		code_hash,
	};

	struct evmc_host_context* context = (struct evmc_host_context*)context_index;
	return evmc_execute(vm, &evmc_go_host, context, rev, &msg, code, code_size);
}

static struct evmc_step_result step_n_wrapper(struct evmc_vm_steppable* vm,
                                              uintptr_t context_index,
                                              enum evmc_revision rev,
                                              enum evmc_call_kind kind,
                                              uint32_t flags,
                                              int32_t depth,
                                              int64_t gas,
                                              const evmc_address* recipient,
                                              const evmc_address* sender,
                                              const uint8_t* input_data,
                                              size_t input_size,
                                              const evmc_uint256be* value,
                                              const evmc_bytes32* code_hash,
                                              const uint8_t* code,
                                              size_t code_size,
                                              enum evmc_step_status_code status,
                                              uint64_t pc,
                                              int64_t gas_refunds,
                                              evmc_uint256be* stack,
                                              size_t stack_size,
                                              uint8_t* memory,
                                              size_t memory_size,
                                              uint8_t* last_call_return_data ,
                                              size_t last_call_return_data_size,
                                              int32_t steps)
{
    struct evmc_message msg = {
        kind,    flags,      depth,      gas,    *recipient,
        *sender, input_data, input_size, *value, {{0}},  // create2_salt: not required for execution
        {{0}},                                           // code_address: not required for execution
        0,                                               // code
        0,                                               // code_size
        code_hash,
    };

    struct evmc_host_context* context = (struct evmc_host_context*)context_index;
    return evmc_step_n(vm, &evmc_go_host, context, rev, &msg, code, code_size, status,
                       pc, gas_refunds, stack, stack_size, memory, memory_size,
					   last_call_return_data, last_call_return_data_size,
					   steps);
}
*/
import "C"

import (
	"fmt"
	"runtime"
	"sync"
	"unsafe"
)

// Hash represents the 32 bytes of arbitrary data (e.g. the result of Keccak256
// hash). It occasionally is used to represent 256-bit unsigned integer values
// stored in big-endian byte order.
type Hash [32]byte

// Address represents the 160-bit (20 bytes) address of an Ethereum account.
type Address [20]byte

// Static asserts.
const (
	// The size of evmc_bytes32 equals the size of Hash.
	_ = uint(len(Hash{}) - C.sizeof_evmc_bytes32)
	_ = uint(C.sizeof_evmc_bytes32 - len(Hash{}))

	// The size of evmc_address equals the size of Address.
	_ = uint(len(Address{}) - C.sizeof_evmc_address)
	_ = uint(C.sizeof_evmc_address - len(Address{}))
)

type Error int32

func (err Error) IsInternalError() bool {
	return err < 0
}

func (err Error) Error() string {
	return C.GoString(C.evmc_status_code_to_string(C.enum_evmc_status_code(err)))
}

const (
	Failure = Error(C.EVMC_FAILURE)
	Revert  = Error(C.EVMC_REVERT)
)

type Revision int32

const (
	Frontier             Revision = C.EVMC_FRONTIER
	Homestead            Revision = C.EVMC_HOMESTEAD
	TangerineWhistle     Revision = C.EVMC_TANGERINE_WHISTLE
	SpuriousDragon       Revision = C.EVMC_SPURIOUS_DRAGON
	Byzantium            Revision = C.EVMC_BYZANTIUM
	Constantinople       Revision = C.EVMC_CONSTANTINOPLE
	Petersburg           Revision = C.EVMC_PETERSBURG
	Istanbul             Revision = C.EVMC_ISTANBUL
	Berlin               Revision = C.EVMC_BERLIN
	London               Revision = C.EVMC_LONDON
	Paris                Revision = C.EVMC_PARIS
	Shanghai             Revision = C.EVMC_SHANGHAI
	Cancun               Revision = C.EVMC_CANCUN
	Prague               Revision = C.EVMC_PRAGUE
	Osaka                Revision = C.EVMC_OSAKA
	Experimental         Revision = C.EVMC_EXPERIMENTAL
	MaxRevision          Revision = C.EVMC_MAX_REVISION
	LatestStableRevision Revision = C.EVMC_LATEST_STABLE_REVISION
)

type VM struct {
	handle *C.struct_evmc_vm
}

func Load(filename string) (vm *VM, err error) {
	cfilename := C.CString(filename)
	loaderErr := C.enum_evmc_loader_error_code(C.EVMC_LOADER_UNSPECIFIED_ERROR)
	handle := C.evmc_load_and_create(cfilename, &loaderErr)
	C.free(unsafe.Pointer(cfilename))

	if loaderErr == C.EVMC_LOADER_SUCCESS {
		vm = &VM{handle}
	} else {
		errMsg := C.evmc_last_error_msg()
		if errMsg != nil {
			err = fmt.Errorf("EVMC loading error: %s", C.GoString(errMsg))
		} else {
			err = fmt.Errorf("EVMC loading error %d", int(loaderErr))
		}
	}

	return vm, err
}

type VMSteppable struct {
	handle *C.struct_evmc_vm_steppable
}

func LoadSteppable(filename string) (vm *VMSteppable, err error) {
	cfilename := C.CString(filename)
	loaderErr := C.enum_evmc_loader_error_code(C.EVMC_LOADER_UNSPECIFIED_ERROR)
	handle := C.evmc_load_and_create_steppable(cfilename, &loaderErr)
	C.free(unsafe.Pointer(cfilename))

	if loaderErr == C.EVMC_LOADER_SUCCESS {
		vm = &VMSteppable{handle}
	} else {
		errMsg := C.evmc_last_error_msg()
		if errMsg != nil {
			err = fmt.Errorf("EVMC loading error: %s", C.GoString(errMsg))
		} else {
			err = fmt.Errorf("EVMC loading error %d", int(loaderErr))
		}
	}

	return vm, err
}

func LoadAndConfigure(config string) (vm *VM, err error) {
	cconfig := C.CString(config)
	loaderErr := C.enum_evmc_loader_error_code(C.EVMC_LOADER_UNSPECIFIED_ERROR)
	handle := C.evmc_load_and_configure(cconfig, &loaderErr)
	C.free(unsafe.Pointer(cconfig))

	if loaderErr == C.EVMC_LOADER_SUCCESS {
		vm = &VM{handle}
	} else {
		errMsg := C.evmc_last_error_msg()
		if errMsg != nil {
			err = fmt.Errorf("EVMC loading error: %s", C.GoString(errMsg))
		} else {
			err = fmt.Errorf("EVMC loading error %d", int(loaderErr))
		}
	}

	return vm, err
}

func (vm *VM) Destroy() {
	C.evmc_destroy(vm.handle)
}

func (vm *VM) Name() string {
	// TODO: consider using C.evmc_vm_name(vm.handle)
	return C.GoString(vm.handle.name)
}

func (vm *VM) Version() string {
	// TODO: consider using C.evmc_vm_version(vm.handle)
	return C.GoString(vm.handle.version)
}

type Capability uint32

const (
	CapabilityEVM1  Capability = C.EVMC_CAPABILITY_EVM1
	CapabilityEWASM Capability = C.EVMC_CAPABILITY_EWASM
)

func (vm *VM) HasCapability(capability Capability) bool {
	return bool(C.evmc_vm_has_capability(vm.handle, uint32(capability)))
}

func (vm *VM) SetOption(name string, value string) (err error) {

	r := C.set_option(vm.handle, C.CString(name), C.CString(value))
	switch r {
	case C.EVMC_SET_OPTION_INVALID_NAME:
		err = fmt.Errorf("evmc: option '%s' not accepted", name)
	case C.EVMC_SET_OPTION_INVALID_VALUE:
		err = fmt.Errorf("evmc: option '%s' has invalid value", name)
	case C.EVMC_SET_OPTION_SUCCESS:
	}
	return err
}

func (vm *VM) GetHandle() unsafe.Pointer {
	return unsafe.Pointer(vm.handle)
}

func (vm *VMSteppable) GetHandle() unsafe.Pointer {
	return unsafe.Pointer(vm.handle)
}

func (vm *VMSteppable) GetBaseHandle() unsafe.Pointer {
	return unsafe.Pointer(vm.handle.vm)
}

func (vm *VMSteppable) Destroy() {
	C.evmc_destroy(vm.handle.vm)
}

type Result struct {
	Output    []byte
	GasLeft   int64
	GasRefund int64
}

func (vm *VM) Execute(ctx HostContext, rev Revision,
	kind CallKind, static bool, delegated bool, depth int, gas int64,
	recipient Address, sender Address, input []byte, value Hash,
	codeHash *Hash, code []byte) (res Result, err error) {

	flags := C.uint32_t(0)
	if static {
		flags |= C.EVMC_STATIC
	}
	if delegated {
		flags |= C.EVMC_DELEGATED
	}

	ctxId := addHostContext(ctx)

	var cCodeHash *C.evmc_bytes32
	if codeHash != nil {
		hash := evmcBytes32(*codeHash)
		cCodeHash = &hash
	}

	txBlobHashes := ctx.GetTxContext().BlobHashes
	if len(txBlobHashes) > 0 {
		pnr := new(runtime.Pinner)
		pnr.Pin(&txBlobHashes[0])
		defer pnr.Unpin()
	}

	// FIXME: Clarify passing by pointer vs passing by value.
	evmcRecipient := evmcAddress(recipient)
	evmcSender := evmcAddress(sender)
	evmcValue := evmcBytes32(value)
	result := C.execute_wrapper(vm.handle, C.uintptr_t(ctxId), uint32(rev),
		C.enum_evmc_call_kind(kind), flags, C.int32_t(depth), C.int64_t(gas),
		&evmcRecipient, &evmcSender, bytesPtr(input), C.size_t(len(input)), &evmcValue,
		cCodeHash, bytesPtr(code), C.size_t(len(code)))
	removeHostContext(ctxId)

	res.Output = C.GoBytes(unsafe.Pointer(result.output_data), C.int(result.output_size))
	res.GasLeft = int64(result.gas_left)
	res.GasRefund = int64(result.gas_refund)
	if result.status_code != C.EVMC_SUCCESS {
		err = Error(result.status_code)
	}

	if result.release != nil {
		C.evmc_release_result(&result)
	}

	return res, err
}

type StepStatus int32

const (
	Running  = StepStatus(C.EVMC_STEP_RUNNING)
	Stopped  = StepStatus(C.EVMC_STEP_STOPPED)
	Returned = StepStatus(C.EVMC_STEP_RETURNED)
	Reverted = StepStatus(C.EVMC_STEP_REVERTED)
	Failed   = StepStatus(C.EVMC_STEP_FAILED)
)

type StepParameters struct {
	Context        HostContext
	Revision       Revision
	Kind           CallKind
	Static         bool
	Depth          int
	Gas            int64
	GasRefund      int64
	Recipient      Address
	Sender         Address
	Input          []byte
	LastCallResult []byte
	Value          Hash
	CodeHash       *Hash
	Code           []byte
	StepStatusCode StepStatus
	Pc             uint64
	Stack          []byte
	Memory         []byte
	NumSteps       int
}

type StepResult struct {
	StepStatusCode     StepStatus
	StatusCode         Error
	Revision           Revision
	Pc                 uint64
	GasLeft            int64
	GasRefund          int64
	Output             []byte
	Stack              []byte
	Memory             []byte
	LastCallReturnData []byte
}

func (vm *VMSteppable) StepN(params StepParameters) (res StepResult, err error) {
	flags := C.uint32_t(0)
	if params.Static {
		flags |= C.EVMC_STATIC
	}

	ctxId := addHostContext(params.Context)
	// FIXME: Clarify passing by pointer vs passing by value.
	evmcRecipient := evmcAddress(params.Recipient)
	evmcSender := evmcAddress(params.Sender)
	evmcValue := evmcBytes32(params.Value)

	var codeHash *C.evmc_bytes32
	if params.CodeHash != nil {
		hash := evmcBytes32(*params.CodeHash)
		codeHash = &hash
	}

	txBlobhashes := params.Context.GetTxContext().BlobHashes
	if len(txBlobhashes) > 0 {
		pnr := new(runtime.Pinner)
		pnr.Pin(&txBlobhashes[0])
		defer pnr.Unpin()
	}

	stack := (*C.evmc_uint256be)(nil)
	if len(params.Stack) > 0 {
		stack = (*C.evmc_uint256be)(unsafe.Pointer(&params.Stack[0]))
	}

	memory := (*C.uint8_t)(nil)
	if len(params.Memory) > 0 {
		memory = (*C.uint8_t)(unsafe.Pointer(&params.Memory[0]))
	}

	result := C.step_n_wrapper(vm.handle,
		C.uintptr_t(ctxId),
		uint32(params.Revision),
		C.enum_evmc_call_kind(params.Kind),
		flags,
		C.int32_t(params.Depth),
		C.int64_t(params.Gas),
		&evmcRecipient,
		&evmcSender,
		bytesPtr(params.Input),
		C.size_t(len(params.Input)),
		&evmcValue,
		codeHash,
		bytesPtr(params.Code),
		C.size_t(len(params.Code)),
		C.enum_evmc_step_status_code(params.StepStatusCode),
		C.uint64_t(params.Pc),
		C.int64_t(params.GasRefund),
		stack,
		C.size_t(len(params.Stack)/32),
		memory,
		C.size_t(len(params.Memory)),
		bytesPtr(params.LastCallResult),
		C.size_t(len(params.LastCallResult)),
		C.int32_t(params.NumSteps))

	removeHostContext(ctxId)

	res.StepStatusCode = StepStatus(result.step_status_code)
	res.StatusCode = Error(result.status_code)
	res.Revision = Revision(result.revision)
	res.Pc = uint64(result.pc)
	res.GasLeft = int64(result.gas_left)
	res.GasRefund = int64(result.gas_refund)
	res.Output = C.GoBytes(unsafe.Pointer(result.output_data), C.int(result.output_size))
	res.Stack = C.GoBytes(unsafe.Pointer(result.stack), C.int(result.stack_size*32))
	res.Memory = C.GoBytes(unsafe.Pointer(result.memory), C.int(result.memory_size))
	res.LastCallReturnData = C.GoBytes(unsafe.Pointer(result.last_call_return_data), C.int(result.last_call_return_data_size))

	if result.release != nil {
		C.evmc_release_step_result(&result)
	}

	return res, err
}

var (
	hostContextCounter uintptr
	hostContextMap     = map[uintptr]HostContext{}
	hostContextMapMu   sync.Mutex
)

func addHostContext(ctx HostContext) uintptr {
	hostContextMapMu.Lock()
	id := hostContextCounter
	hostContextCounter++
	hostContextMap[id] = ctx
	hostContextMapMu.Unlock()
	return id
}

func removeHostContext(id uintptr) {
	hostContextMapMu.Lock()
	delete(hostContextMap, id)
	hostContextMapMu.Unlock()
}

func getHostContext(idx uintptr) HostContext {
	hostContextMapMu.Lock()
	ctx := hostContextMap[idx]
	hostContextMapMu.Unlock()
	return ctx
}

func evmcBytes32(in Hash) C.evmc_bytes32 {
	out := C.evmc_bytes32{}
	for i := 0; i < len(in); i++ {
		out.bytes[i] = C.uint8_t(in[i])
	}
	return out
}

func evmcAddress(address Address) C.evmc_address {
	r := C.evmc_address{}
	for i := 0; i < len(address); i++ {
		r.bytes[i] = C.uint8_t(address[i])
	}
	return r
}

func bytesPtr(bytes []byte) *C.uint8_t {
	if len(bytes) == 0 {
		return nil
	}
	return (*C.uint8_t)(unsafe.Pointer(&bytes[0]))
}
