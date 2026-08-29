// Copyright 2026 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;

use serde_test::Configure;
use serde_test::Token;
use serde_test::assert_ser_tokens;

#[test]
fn byte_slices_use_serde_sequence_calls() {
    let bytes = [1_u8, 2];
    assert_ser_tokens(
        &&bytes[..],
        &[
            Token::Seq { len: Some(2) },
            Token::U8(1),
            Token::U8(2),
            Token::SeqEnd,
        ],
    );
}

#[test]
fn network_types_switch_between_readable_and_compact_calls() {
    let ipv4 = Ipv4Addr::new(127, 0, 0, 1);
    assert_ser_tokens(&ipv4.readable(), &[Token::Str("127.0.0.1")]);
    assert_ser_tokens(
        &ipv4.compact(),
        &[
            Token::Tuple { len: 4 },
            Token::U8(127),
            Token::U8(0),
            Token::U8(0),
            Token::U8(1),
            Token::TupleEnd,
        ],
    );

    let socket = SocketAddr::V4(SocketAddrV4::new(ipv4, 8080));
    assert_ser_tokens(&socket.readable(), &[Token::Str("127.0.0.1:8080")]);
    assert_ser_tokens(
        &socket.compact(),
        &[
            Token::NewtypeVariant {
                name: "SocketAddr",
                variant: "V4",
            },
            Token::Tuple { len: 2 },
            Token::Tuple { len: 4 },
            Token::U8(127),
            Token::U8(0),
            Token::U8(0),
            Token::U8(1),
            Token::TupleEnd,
            Token::U16(8080),
            Token::TupleEnd,
        ],
    );
}
