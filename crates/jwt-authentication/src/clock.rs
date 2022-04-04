// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

use chrono::Utc;

/// The clock. Should return the current timestamp.
pub trait Clock {
    /// Returns the number of non-leap seconds since January 1, 1970 0:00:00 UTC (aka "UNIX timestamp").
    fn timestamp(&self) -> i64;
}

impl Clock for Utc {
    fn timestamp(&self) -> i64 {
        Utc::now().timestamp()
    }
}
