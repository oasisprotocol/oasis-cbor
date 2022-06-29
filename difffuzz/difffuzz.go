// Package difffuzz implements a differential fuzzer to ensure Go/Rust compatibility.
package difffuzz

//#cgo CFLAGS: -I.
//#cgo LDFLAGS: -Ltarget/release -ldifffuzz
//#include <difffuzz.h>
import "C"

import (
	"fmt"
	"unsafe"
)

func CborFromSlice(data []byte) error {
	ptr := (*C.uchar)(unsafe.Pointer(&data[0]))
	len := C.size_t(len(data))
	result := C.cbor_from_slice(ptr, len)
	if result != 0 {
		return fmt.Errorf("error during decoding")
	}

	return nil
}
