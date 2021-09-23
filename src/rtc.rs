/*!
  # Real Time Clock
  A clock that constantly counts up starting at the time of it's creation.

  ## Initialisation example
  ```rust
    let rtc = Rtc::new(dp.HBN);
  ```
*/

use bl602_pac::HBN;
use embedded_time::Clock;

pub struct Rtc {
    hbn: HBN,
}

impl Rtc {
    /// Creates and starts the RTC
    pub fn new(hbn: HBN) -> Rtc {
        // clear counter
        hbn.hbn_ctl
            .modify(|r, w| unsafe { w.rtc_ctl().bits(r.rtc_ctl().bits() & 0xfe) });
        // enable counter
        hbn.hbn_ctl
            .modify(|r, w| unsafe { w.rtc_ctl().bits(r.rtc_ctl().bits() | 1) });

        Rtc { hbn }
    }

    /// Get elapsed milliseconds since the RTC was created
    pub fn get_millis(&self) -> u64 {
        self.hbn
            .rtc_time_h
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << 31) });

        let h = self.hbn.rtc_time_h.read().bits();
        let l = self.hbn.rtc_time_l.read().bits();
        let ts = (h as u64) << 32 | l as u64; // in counter units

        // from IOT SDK:
        // #define BL_RTC_COUNTER_TO_MS(CNT)  (((CNT) >> 5) - ((CNT) >> 11) - ((CNT) >> 12))  // ((CNT)*(1024-16-8)/32768)
        // see https://github.com/bouffalolab/bl_iot_sdk/blob/90acb7b46d11343d27db9518c4f86d94572c6629/components/hal_drv/bl602_hal/bl_rtc.c
        ts * (1024 - 16 - 8) / 32768
    }
}

impl Clock for Rtc {
    type T = u64;

    const SCALING_FACTOR: embedded_time::fraction::Fraction =
        <embedded_time::fraction::Fraction>::new(1, 1_000);

    fn try_now(&self) -> Result<embedded_time::Instant<Self>, embedded_time::clock::Error> {
        Ok(embedded_time::Instant::new(self.get_millis()))
    }
}
