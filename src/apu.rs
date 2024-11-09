use crate::config::Speed;
use crate::context;

use log::warn;
use modular_bitfield::prelude::*;

const CYCLES_PER_FRAME: u32 = 70224;
const SAMPLE_PER_FRAME: u32 = 800;

trait Context: context::Config {}
impl<T> Context for T where T: context::Config {}

#[derive(Debug, Default)]
pub struct Apu {
    is_on: bool,
    audio_buffer: Vec<[i16; 2]>,

    pulse: [Pulse; 2],
    wave: Wave,
    noise: Noise,

    master_volume: MasterVolume, // 0xFF24
    panning: [[bool; 4]; 2],     // 0xFF25

    frame_sequencer: FrameSequencer,
    sample_counter: u32,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            pulse: [Pulse::new(), Pulse::new()],

            frame_sequencer: FrameSequencer::new(), // 512 Hz

            ..Default::default()
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF10..=0xFF14 => {
                let offset = address - 0xFF10;
                self.pulse[0].read(offset)
            }
            0xFF16..=0xFF19 => {
                let offset = address - 0xFF16;
                self.pulse[1].read(offset)
            }
            0xFF24 => self.master_volume.bytes[0],
            0xFF25 => {
                let mut ret = 0;
                for i in 0..2 {
                    for j in 0..4 {
                        ret |= (self.panning[i][j] as u8) << (i * 4 + j);
                    }
                }
                ret
            }
            0xFF26 => {
                let mut ret = 0;
                ret |= self.pulse[0].is_on as u8;
                ret |= (self.pulse[1].is_on as u8) << 1;
                ret |= (self.wave.is_on as u8) << 2;
                ret |= (self.noise.is_on as u8) << 3;
                ret |= (self.is_on as u8) << 7;
                ret
            }
            _ => {
                warn!("Apu read not implemented: {:#06X}", address);
                0x00
            }
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF10..=0xFF14 => {
                let offset = address - 0xFF10;
                self.pulse[0].write(offset, value);
            }
            0xFF16..=0xFF19 => {
                let offset = address - 0xFF16;
                self.pulse[1].write(offset, value);
            }
            0xFF24 => self.master_volume = MasterVolume::from_bytes([value]),
            0xFF25 => {
                for i in 0..2 {
                    for j in 0..4 {
                        self.panning[i][j] = (value >> (i * 4 + j)) & 1 == 1;
                    }
                }
            }
            0xFF26 => self.is_on = (value >> 7) & 1 == 1,
            _ => warn!("Apu write not implemented: {:#06X}", address),
        }
    }

    pub fn tick(&mut self, context: &impl Context) {
        let tick_count = match context.current_speed() {
            Speed::Normal => 4,
            Speed::Double => 2,
        };
        for _ in 0..tick_count {
            self.tick_();
        }
    }

    pub fn tick_(&mut self) {
        if self.is_on {
            let (should_length_tick, should_volume_tick, should_sweep_tick) =
                self.frame_sequencer.tick();

            self.pulse[0].tick(should_length_tick, should_volume_tick, should_sweep_tick);
            self.pulse[1].tick(should_length_tick, should_volume_tick, false);
            // self.wave.tick();
            // self.noise.tick();
        }

        self.sample_counter += SAMPLE_PER_FRAME;
        if self.sample_counter >= CYCLES_PER_FRAME {
            self.sample_counter -= CYCLES_PER_FRAME;
            let output = self.mix_output();
            self.audio_buffer.push(output);
        }
    }

    fn mix_output(&self) -> [i16; 2] {
        if !self.is_on {
            return [0, 0];
        }

        let channel_output = [
            self.pulse[0].output(),
            self.pulse[1].output(),
            // self.wave.output(),
            // self.noise.output(),
        ];
        let mut output = [0, 0];

        for i in 0..2 {
            for (ch_idx, ch_output) in channel_output.iter().enumerate() {
                if self.panning[i][ch_idx] {
                    output[i] += ch_output;
                }
            }
            if i == 0 {
                output[i] = (output[i] * self.master_volume.left_volume() as i16) >> 3;
            } else {
                output[i] = (output[i] * self.master_volume.right_volume() as i16) >> 3;
            }
        }

        output
    }

    pub fn get_audio_buffer(&self) -> &Vec<[i16; 2]> {
        &self.audio_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
    }
}

static WAVEFORM: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

#[derive(Debug, Default)]
struct Pulse {
    is_on: bool,

    sweep: Sweep,                          // 0xFF10
    length_timer: u8,                      // 0xFF11, 0xFF16 (bit:0-5)
    wave_duty: u8,                         // 0xFF11, 0xFF16 (bit:6-7)
    envelope_period: u8,                   // 0xFF12, 0xFF17 (bit:0-2)
    envelope_direction: EnvelopeDirection, // 0xFF12, 0xFF17 (bit:3)
    initial_volume: u8,                    // 0xFF12, 0xFF17 (bit:4-7)
    frequency: u16,                        // 0xFF13, 0xFF14, 0xFF18, 0xFF19
    length_enable: bool,                   // 0xFF14, 0xFF19 (bit:6)

    current_volume: u8,
    current_frequency: u16,
    frequency_timer: u16,
    envelope_timer: u8,
    sweep_timer: u8,
    sweep_enable: bool,
    phase: usize,
}

impl Pulse {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn read(&self, offset: u16) -> u8 {
        match offset {
            0 => self.sweep.bytes[0],
            1 => self.wave_duty << 6 | 0x3F,
            2 => {
                (self.initial_volume << 4)
                    | (self.envelope_direction as u8) << 3
                    | self.envelope_period
            }
            3 => 0xFF,
            4 => (self.length_enable as u8) << 6 | 0b1011_1111,

            _ => unreachable!("Pulse invalid read offset: {:#06X}", offset),
        }
    }

    fn write(&mut self, offset: u16, value: u8) {
        match offset {
            0 => self.sweep = Sweep::from_bytes([value]),
            1 => {
                self.wave_duty = value >> 6;
                self.length_timer = 64 - (value & 0x3F);
            }
            2 => {
                self.envelope_period = value & 0x07;
                self.envelope_direction = EnvelopeDirection::from(value >> 3 & 1);
                self.initial_volume = value >> 4;
            }
            3 => self.frequency = (self.frequency & 0x0700) | value as u16,
            4 => {
                self.frequency = (self.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                self.length_enable = (value >> 6) & 1 == 1;
                if value >> 7 & 1 == 1 {
                    self.trigger();
                }
            }

            _ => unreachable!("Pulse invalid write offset: {:#06X}", offset),
        }
    }

    fn tick(
        &mut self,
        should_length_tick: bool,
        should_volume_tick: bool,
        should_sweep_tick: bool,
    ) {
        self.frequency_timer = self.frequency_timer.saturating_sub(1);
        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency) * 4;
            self.phase = (self.phase + 1) % 8;
        }

        if should_length_tick {
            self.length_tick();
        }
        if should_volume_tick {
            self.envelope_tick();
        }
        if should_sweep_tick {
            self.sweep_tick();
        }
    }

    fn length_tick(&mut self) {
        self.length_timer = (self.length_timer + 1).min(64);
        if self.length_timer == 64 {
            self.is_on = false;
        }
    }

    fn envelope_tick(&mut self) {
        if self.envelope_timer > 0 {
            self.envelope_timer -= 1;
            if self.envelope_timer == 0 && self.envelope_period != 0 {
                self.envelope_timer = self.envelope_period;
                self.current_volume = match self.envelope_direction {
                    EnvelopeDirection::Decrease => self.current_volume.saturating_sub(1),
                    EnvelopeDirection::Increase => (self.current_volume + 1).min(15),
                };
            }
        }
    }

    fn sweep_tick(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
            if self.sweep_timer == 0 && self.sweep.period() != 0 {
                self.sweep_timer = self.sweep.period();
                let new_frequency = self.new_frequency();
                if new_frequency <= 2047 && self.sweep.shift() != 0 {
                    self.current_frequency = new_frequency;
                    self.frequency = new_frequency;
                }

                if self.new_frequency() > 2047 {
                    self.is_on = false;
                }
            }
        }
    }

    fn new_frequency(&self) -> u16 {
        match self.sweep.direction() {
            SweepDirection::Addition => {
                self.current_frequency + (self.current_frequency >> self.sweep.shift())
            }
            SweepDirection::Subtraction => {
                self.current_frequency - (self.current_frequency >> self.sweep.shift())
            }
        }
    }

    fn trigger(&mut self) {
        self.is_on =
            self.initial_volume != 0 || self.envelope_direction == EnvelopeDirection::Increase;

        if self.length_timer == 0 {
            self.length_timer = 64;
        }
        self.frequency_timer = (2048 - self.frequency) * 4;
        self.envelope_timer = if self.envelope_period == 0 {
            8
        } else {
            self.envelope_period
        };

        self.current_volume = self.initial_volume;
        self.current_frequency = self.frequency;

        self.sweep_timer = if self.sweep.period() == 0 {
            8
        } else {
            self.sweep.period()
        };
        self.sweep_enable = self.sweep.period() != 0 || self.sweep.shift() != 0;
        if self.sweep.shift() != 0 && self.new_frequency() > 2047 {
            self.is_on = false;
        }
    }

    fn output(&self) -> i16 {
        if self.is_on {
            let waveform = WAVEFORM[self.wave_duty as usize][self.phase] as i16 * 2 - 1;
            let volume = self.current_volume as i16;
            waveform * volume * 256
        } else {
            0
        }
    }
}

#[derive(Debug, Default)]
struct Wave {
    is_on: bool,
}

impl Wave {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, address: u16, value: u8) {
        todo!()
    }

    fn length_tick(&mut self) {
        todo!()
    }
}

#[derive(Debug, Default)]
struct Noise {
    is_on: bool,
}

impl Noise {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, address: u16, value: u8) {
        todo!()
    }

    fn length_tick(&mut self) {
        todo!()
    }
}

#[bitfield(bits = 8)]
#[derive(Debug, Default)]
struct MasterVolume {
    right_volume: B3,
    right_vin: bool,
    left_volume: B3,
    left_vin: bool,
}

#[bitfield(bits = 8)]
#[derive(Debug, Default)]
struct Sweep {
    shift: B3,
    direction: SweepDirection,
    period: B3,
    #[skip]
    _unused: B1,
}

#[derive(BitfieldSpecifier, Debug, Default)]
enum SweepDirection {
    #[default]
    Addition = 0,
    Subtraction = 1,
}

#[repr(u8)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
enum EnvelopeDirection {
    #[default]
    Decrease = 0,
    Increase = 1,
}

impl From<u8> for EnvelopeDirection {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Decrease,
            1 => Self::Increase,
            _ => unreachable!("Invalid EnvelopeDirection: {}", value),
        }
    }
}

#[derive(Debug, Default)]
struct FrameSequencer {
    counter: u32,
    step: u8,
}

impl FrameSequencer {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn tick(&mut self) -> (bool, bool, bool) {
        let mut should_length_tick = false;
        let mut should_volume_tick = false;
        let mut should_sweep_tick = false;

        self.counter += 1;
        if self.counter >= 8192 {
            self.counter = 0;
            self.step = (self.step + 1) % 8;

            if self.step % 2 == 0 {
                should_length_tick = true;
            }
            if self.step == 7 {
                should_volume_tick = true;
            }
            if self.step == 2 || self.step == 6 {
                should_sweep_tick = true;
            }
        }

        (should_length_tick, should_volume_tick, should_sweep_tick)
    }
}
