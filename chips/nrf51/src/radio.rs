//! Radio driver, Bluetooth Low Energy, nRF51
//!
//! Sending Bluetooth Low Energy advertisement packets with payloads up to 31 bytes
//!
//! Currently all fields in PAYLOAD array are configurable from user-space
//! except the PDU_TYPE.
//!
//! ### Authors
//! * Niklas Adolfsson <niklasadolfsson1@gmail.com>
//! * Fredrik Nilsson <frednils@student.chalmers.se>
//! * Date: June 22, 2017

use core::cell::Cell;
use core::convert::TryFrom;
use kernel;
use kernel::common::cells::OptionalCell;
use kernel::common::cells::VolatileCell;
use kernel::common::StaticRef;
use kernel::hil::ble_advertising;
use kernel::hil::ble_advertising::RadioChannel;
use kernel::ReturnCode;
use nrf5x;
use nrf5x::constants::TxPower;

pub static mut RADIO: Radio = Radio::new();

static mut PAYLOAD: [u8; nrf5x::constants::RADIO_PAYLOAD_LENGTH] =
    [0x00; nrf5x::constants::RADIO_PAYLOAD_LENGTH];

#[repr(C)]
struct RadioRegisters {
    txen: VolatileCell<u32>,        // 0x000 ---> 0x004
    rxen: VolatileCell<u32>,        // 0x004 ---> 0x008
    start: VolatileCell<u32>,       // 0x008 ---> 0x00c
    stop: VolatileCell<u32>,        // 0x00c ---> 0x010
    disable: VolatileCell<u32>,     // 0x010 ---> 0x014
    rssistart: VolatileCell<u32>,   // 0x014 ---> 0x018
    rssistop: VolatileCell<u32>,    // 0x018 ---> 0x01c
    bcstart: VolatileCell<u32>,     // 0x01c ---> 0x020
    bcstop: VolatileCell<u32>,      // 0x020 ---> 0x024
    _reserved1: [u32; 55],          // 0x024 ---> 0x100
    ready: VolatileCell<u32>,       // 0x100 ---> 0x104
    address: VolatileCell<u32>,     // 0x104 ---> 0x108
    payload: VolatileCell<u32>,     // 0x108 ---> 0x10c
    end: VolatileCell<u32>,         // 0x10c ---> 0x110
    disabled: VolatileCell<u32>,    // 0x110 ---> 0x114
    devmatch: VolatileCell<u32>,    // 0x114 ---> 0x118
    devmiss: VolatileCell<u32>,     // 0x118 ---> 0x11c
    rssiend: VolatileCell<u32>,     // 0x11c -->  0x120
    _reserved2: [u32; 2],           // 0x120 ---> 0x128
    bcmatch: VolatileCell<u32>,     // 0x128 ---> 0x12c
    _reserved3: [u32; 53],          // 0x12c ---> 0x200
    shorts: VolatileCell<u32>,      // 0x200 ---> 0x204
    _reserved4: [u32; 64],          // 0x204 ---> 0x304
    intenset: VolatileCell<u32>,    // 0x304 ---> 0x308
    intenclr: VolatileCell<u32>,    // 0x308 ---> 0x30c
    _reserved5: [u32; 61],          // 0x30c ---> 0x400
    crcstatus: VolatileCell<u32>,   // 0x400 - 0x404
    _reserved6: [u32; 1],           // 0x404 - 0x408
    rxmatch: VolatileCell<u32>,     // 0x408 - 0x40c
    rxcrc: VolatileCell<u32>,       // 0x40c - 0x410
    dai: VolatileCell<u32>,         // 0x410 - 0x414
    _reserved7: [u32; 60],          // 0x414 - 0x504
    packetptr: VolatileCell<u32>,   // 0x504 - 0x508
    frequency: VolatileCell<u32>,   // 0x508 - 0x50c
    txpower: VolatileCell<u32>,     // 0x50c - 0x510
    mode: VolatileCell<u32>,        // 0x510 - 0x514
    pcnf0: VolatileCell<u32>,       // 0x514 - 0x518
    pcnf1: VolatileCell<u32>,       // 0x518 - 0x51c
    base0: VolatileCell<u32>,       // 0x51c - 0x520
    base1: VolatileCell<u32>,       // 0x520 - 0x524
    prefix0: VolatileCell<u32>,     // 0x524 - 0x528
    prefix1: VolatileCell<u32>,     // 0x528 - 0x52c
    txaddress: VolatileCell<u32>,   // 0x52c - 0x530
    rxaddresses: VolatileCell<u32>, // 0x530 - 0x534
    crccnf: VolatileCell<u32>,      // 0x534 - 0x538
    crcpoly: VolatileCell<u32>,     // 0x538 - 0x53c
    crcinit: VolatileCell<u32>,     // 0x53c - 0x540
    test: VolatileCell<u32>,        // 0x540 - 0x544
    tifs: VolatileCell<u32>,        // 0x544 - 0x548
    rssisample: VolatileCell<u32>,  // 0x548 - 0x54c
    _reserved8: [u32; 1],           // 0x54c - 0x550
    state: VolatileCell<u32>,       // 0x550 - 0x554
    datawhiteiv: VolatileCell<u32>, // 0x554 - 0x558
    _reserved9: [u32; 2],           // 0x558 - 0x560
    bcc: VolatileCell<u32>,         // 0x560 - 0x564
    _reserved10: [u32; 39],         // 0x560 - 0x600
    dab0: VolatileCell<u32>,        // 0x600 - 0x604
    dab1: VolatileCell<u32>,        // 0x604 - 0x608
    dab2: VolatileCell<u32>,        // 0x608 - 0x60c
    dab3: VolatileCell<u32>,        // 0x60c - 0x610
    dab4: VolatileCell<u32>,        // 0x610 - 0x614
    dab5: VolatileCell<u32>,        // 0x614 - 0x618
    dab6: VolatileCell<u32>,        // 0x618 - 0x61c
    dab7: VolatileCell<u32>,        // 0x61c - 0x620
    dap0: VolatileCell<u32>,        // 0x620 - 0x624
    dap1: VolatileCell<u32>,        // 0x624 - 0x628
    dap2: VolatileCell<u32>,        // 0x628 - 0x62c
    dap3: VolatileCell<u32>,        // 0x62c - 0x630
    dap4: VolatileCell<u32>,        // 0x630 - 0x634
    dap5: VolatileCell<u32>,        // 0x634 - 0x638
    dap6: VolatileCell<u32>,        // 0x638 - 0x63c
    dap7: VolatileCell<u32>,        // 0x63c - 0x640
    dacnf: VolatileCell<u32>,       // 0x640 - 0x644
    _reserved11: [u32; 56],         // 0x644 - 0x724
    override0: VolatileCell<u32>,   // 0x724 - 0x728
    override1: VolatileCell<u32>,   // 0x728 - 0x72c
    override2: VolatileCell<u32>,   // 0x72c - 0x730
    override3: VolatileCell<u32>,   // 0x730 - 0x734
    override4: VolatileCell<u32>,   // 0x734 - 0x738
    _reserved12: [u32; 561],        // 0x738 - 0x724
    power: VolatileCell<u32>,       // 0xFFC - 0x1000
}

const RADIO_BASE: StaticRef<RadioRegisters> =
    unsafe { StaticRef::new(0x40001000 as *const RadioRegisters) };

pub struct Radio {
    registers: StaticRef<RadioRegisters>,
    tx_power: Cell<TxPower>,
    rx_client: OptionalCell<&'static ble_advertising::RxClient>,
    tx_client: OptionalCell<&'static ble_advertising::TxClient>,
}

impl Radio {
    pub const fn new() -> Radio {
        Radio {
            registers: RADIO_BASE,
            tx_power: Cell::new(TxPower::ZerodBm),
            rx_client: OptionalCell::empty(),
            tx_client: OptionalCell::empty(),
        }
    }

    fn ble_initialize(&self, channel: RadioChannel) {
        let regs = &*self.registers;

        self.radio_on();

        // TX Power acc. to twpower variable in the struct
        self.set_tx_power();

        // BLE MODE
        self.set_channel_rate(nrf5x::constants::RadioMode::Ble1Mbit as u32);

        self.set_channel_freq(channel);
        self.set_data_whitening(channel);

        // Set PREFIX | BASE Address
        regs.prefix0.set(0x0000008e);
        regs.base0.set(0x89bed600);

        self.set_tx_address(0x00);
        self.set_rx_address(0x01);

        // Set Packet Config
        self.set_packet_config(0x00);

        // CRC Config
        self.set_crc_config();

        // Buffer configuration
        self.set_dma_ptr();
    }

    fn tx(&self) {
        let regs = &*self.registers;
        regs.ready.set(0);
        regs.txen.set(1);
    }

    fn rx(&self) {
        let regs = &*self.registers;
        regs.ready.set(0);
        regs.rxen.set(1);
    }

    fn set_crc_config(&self) {
        let regs = &*self.registers;
        regs.crccnf.set(
            nrf5x::constants::RADIO_CRCCNF_LEN_3BYTES
                | nrf5x::constants::RADIO_CRCCNF_SKIPADDR
                    << nrf5x::constants::RADIO_CRCCNF_SKIPADDR_POS,
        );
        regs.crcinit.set(nrf5x::constants::RADIO_CRCINIT_BLE);
        regs.crcpoly.set(nrf5x::constants::RADIO_CRCPOLY_BLE);
    }

    // Packet configuration
    fn set_packet_config(&self, _: u32) {
        let regs = &*self.registers;
        regs.pcnf0.set(
            (nrf5x::constants::RADIO_PCNF0_S0_LEN_1BYTE << nrf5x::constants::RADIO_PCNF0_S0LEN_POS)
                | (nrf5x::constants::RADIO_PCNF0_LFLEN_1BYTE
                    << nrf5x::constants::RADIO_PCNF0_LFLEN_POS),
        );

        regs.pcnf1.set(
            (nrf5x::constants::RADIO_PCNF1_WHITEEN_ENABLED <<
                nrf5x::constants::RADIO_PCNF1_WHITEEN_POS) |
                 (nrf5x::constants::RADIO_PCNF1_ENDIAN_LITTLE <<
                     nrf5x::constants::RADIO_PCNF1_ENDIAN_POS) |
                 // Total Address is 4 bytes (BASE ADDRESS + PREFIX (1))
                 (nrf5x::constants::RADIO_PCNF1_BALEN_3BYTES <<
                  nrf5x::constants::RADIO_PCNF1_BALEN_POS)
                | (nrf5x::constants::RADIO_PCNF1_STATLEN_DONT_EXTEND
                    << nrf5x::constants::RADIO_PCNF1_STATLEN_POS)
                | (nrf5x::constants::RADIO_PCNF1_MAXLEN_37BYTES
                    << nrf5x::constants::RADIO_PCNF1_MAXLEN_POS),
        );
    }

    fn set_rx_address(&self, _: u32) {
        let regs = &*self.registers;
        regs.rxaddresses.set(0x01);
    }

    fn set_tx_address(&self, _: u32) {
        let regs = &*self.registers;
        regs.txaddress.set(0x00);
    }

    fn set_channel_rate(&self, rate: u32) {
        let regs = &*self.registers;
        // set channel rate,  3 - BLE 1MBIT/s
        regs.mode.set(rate);
    }

    fn set_data_whitening(&self, channel: RadioChannel) {
        let regs = &*self.registers;
        regs.datawhiteiv.set(channel.get_channel_index());
    }

    fn set_channel_freq(&self, channel: RadioChannel) {
        let regs = &*self.registers;
        //37, 38 and 39 for adv.
        regs.frequency.set(channel as u32);
    }

    fn radio_on(&self) {
        let regs = &*self.registers;
        // reset and enable power
        regs.power.set(0);
        regs.power.set(1);
    }

    fn radio_off(&self) {
        let regs = &*self.registers;
        regs.power.set(0);
    }

    // pre-condition validated before arriving here
    fn set_tx_power(&self) {
        let regs = &*self.registers;
        regs.txpower.set(self.tx_power.get() as u32);
    }

    fn set_dma_ptr(&self) {
        let regs = &*self.registers;
        unsafe {
            regs.packetptr.set(PAYLOAD.as_ptr() as u32);
        }
    }

    #[inline(never)]
    pub fn handle_interrupt(&self) {
        let regs = &*self.registers;
        self.disable_interrupts();

        if regs.ready.get() == 1 {
            regs.ready.set(0);
            regs.end.set(0);
            regs.start.set(1);
        }

        if regs.payload.get() == 1 {
            regs.payload.set(0);
        }

        if regs.address.get() == 1 {
            regs.address.set(0);
        }

        if regs.end.get() == 1 {
            regs.end.set(0);
            regs.disable.set(1);

            let result = if regs.crcstatus.get() == 1 {
                ReturnCode::SUCCESS
            } else {
                ReturnCode::FAIL
            };

            match regs.state.get() {
                nrf5x::constants::RADIO_STATE_TXRU
                | nrf5x::constants::RADIO_STATE_TXIDLE
                | nrf5x::constants::RADIO_STATE_TXDISABLE
                | nrf5x::constants::RADIO_STATE_TX => {
                    self.radio_off();
                    self.tx_client.map(|client| client.transmit_event(result));
                }
                nrf5x::constants::RADIO_STATE_RXRU
                | nrf5x::constants::RADIO_STATE_RXIDLE
                | nrf5x::constants::RADIO_STATE_RXDISABLE
                | nrf5x::constants::RADIO_STATE_RX => {
                    self.radio_off();
                    unsafe {
                        self.rx_client.map(|client| {
                            // Length is: S0 (1 Byte) + Length (1 Byte) + S1 (0 Bytes) + Payload
                            // And because the length field is directly read from the packet
                            // We need to add 2 to length to get the total length
                            client.receive_event(&mut PAYLOAD, PAYLOAD[1] + 2, result)
                        });
                    }
                }
                // Radio state - Disabled
                _ => (),
            }
        }
        self.enable_interrupts();
    }

    pub fn enable_interrupts(&self) {
        let regs = &*self.registers;
        regs.intenset.set(
            nrf5x::constants::RADIO_INTENSET_READY
                | nrf5x::constants::RADIO_INTENSET_ADDRESS
                | nrf5x::constants::RADIO_INTENSET_PAYLOAD
                | nrf5x::constants::RADIO_INTENSET_END,
        );
    }

    pub fn disable_interrupts(&self) {
        let regs = &*self.registers;
        // disable all possible interrupts
        regs.intenclr.set(0xffffffff);
    }

    pub fn replace_radio_buffer(&self, buf: &'static mut [u8], len: usize) -> &'static mut [u8] {
        // set payload
        for (i, c) in buf.as_ref()[0..len].iter().enumerate() {
            unsafe {
                PAYLOAD[i] = *c;
            }
        }
        buf
    }
}

impl ble_advertising::BleAdvertisementDriver for Radio {
    fn transmit_advertisement(
        &self,
        buf: &'static mut [u8],
        len: usize,
        channel: RadioChannel,
    ) -> &'static mut [u8] {
        let res = self.replace_radio_buffer(buf, len);
        self.ble_initialize(channel);
        self.tx();
        self.enable_interrupts();
        res
    }

    fn receive_advertisement(&self, channel: RadioChannel) {
        self.ble_initialize(channel);
        self.rx();
        self.enable_interrupts();
    }

    fn set_receive_client(&self, client: &'static ble_advertising::RxClient) {
        self.rx_client.set(client);
    }

    fn set_transmit_client(&self, client: &'static ble_advertising::TxClient) {
        self.tx_client.set(client);
    }
}

impl ble_advertising::BleConfig for Radio {
    // The BLE Advertising Driver validates that the `tx_power` is between -20 to 10 dBm but then
    // underlying chip must validate if the current `tx_power` is supported as well
    fn set_tx_power(&self, tx_power: u8) -> kernel::ReturnCode {
        // Convert u8 to TxPower
        match nrf5x::constants::TxPower::try_from(tx_power) {
            // Invalid transmitting power, propogate error
            Err(_) => kernel::ReturnCode::ENOSUPPORT,
            // Valid transmitting power, propogate success
            Ok(res) => {
                self.tx_power.set(res);
                kernel::ReturnCode::SUCCESS
            }
        }
    }
}
