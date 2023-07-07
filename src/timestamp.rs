pub struct Timestamps {
    created: Timestamp,
    modified: Timestamp,
    accessed: Timestamp,
}

impl Timestamps {
    pub fn new(created: Timestamp, modified: Timestamp, accessed: Timestamp) -> Self {
        Timestamps {
            created,
            modified,
            accessed,
        }
    }

    pub fn created(&self) -> &Timestamp {
        &self.created
    }

    pub fn modified(&self) -> &Timestamp {
        &self.modified
    }

    pub fn accessed(&self) -> &Timestamp {
        &self.accessed
    }
}

pub struct Timestamp {
    timestamp: u32,
    ms_increment: u8,
    // Offset from UTC in 15 minute intervals
    utc_offset: i8,
}

pub struct Date {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

// See https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification#748-timestamp-fields
impl Timestamp {
    pub fn new(timestamp: u32, ms_increment: u8, utc_offset: i8) -> Self {
        Timestamp {
            timestamp,
            ms_increment,
            utc_offset,
        }
    }

    pub fn date(&self) -> Date {
        Date {
            day: ((self.timestamp >> 16) & 0x1F) as u8,
            month: ((self.timestamp >> 21) & 0xF) as u8,
            year: 1980 + ((self.timestamp >> 25) & 0x7F) as u16,
        }
    }

    pub fn time(&self) -> Time {
        Time {
            second: (self.ms_increment as u16 / 1000) as u8 + (self.timestamp & 0x1F) as u8 * 2,
            minute: ((self.timestamp >> 5) & 0x3f) as u8,
            hour: ((self.timestamp >> 11) & 0x1F) as u8,
        }
    }

    pub fn utc_offset(&self) -> i8 {
        self.utc_offset
    }
}
