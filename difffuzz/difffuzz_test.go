package difffuzz

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/oasisprotocol/oasis-core/go/common/cbor"
)

func TestDecodingFromRust(t *testing.T) {
	err := CborFromSlice([]byte{0x81, 0x18, 0x2a})
	require.NoError(t, err)

	err = CborFromSlice([]byte{0xde, 0xad, 0xbe, 0xef})
	require.Error(t, err)
}

func FuzzDifferential(f *testing.F) {
	// Seed corpus.
	f.Add([]byte{0x81, 0x18, 0x2a})
	f.Add([]byte{0x18, 0x2a})
	f.Add([]byte{0xA1, 0x63, 0x66, 0x6F, 0x6F, 0x0A})
	f.Add([]byte{0xA2, 0x65, 0x62, 0x79, 0x74, 0x65, 0x73, 0x41, 0x01, 0x63, 0x66, 0x6F, 0x6F, 0x18, 0x2A})

	// Fuzzing.
	f.Fuzz(func(t *testing.T, data []byte) {
		var output map[interface{}]interface{}
		err := cbor.Unmarshal(data, &output)
		if err != nil {
			return
		}

		// If decoding succeeded, make sure it also succeeds in the Rust version.
		err = CborFromSlice(data)
		if err != nil {
			t.Logf("data: %X", data)
			panic("decoding passed in Go but failed in Rust")
		}
	})
}
