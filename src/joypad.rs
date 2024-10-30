use crate::context;
use bitflags::bitflags;

trait Context: context::Interrupt {}
impl<T> Context for T where T: context::Interrupt {}

bitflags! {
    #[derive(Default, Clone, Copy)]
    struct Keys: u8 {
        const RIGHT  = 0b0000_0001;
        const LEFT   = 0b0000_0010;
        const UP     = 0b0000_0100;
        const DOWN   = 0b0000_1000;
        const A      = 0b0001_0000;
        const B      = 0b0010_0000;
        const SELECT = 0b0100_0000;
        const START  = 0b1000_0000;
    }
}

pub struct Joypad {
    key_state: JoypadKeyState,
    direction_selected: bool,
    action_selected: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            key_state: JoypadKeyState::new(),
            direction_selected: false,
            action_selected: false,
        }
    }

    pub fn read(&self) -> u8 {
        let mut ret = 0xCF;

        if self.direction_selected {
            ret &= !0x10; // ビット4を0に設定（P14選択）
            ret = (ret & 0xF0) | self.key_state.get_direction();
        }
        if self.action_selected {
            ret &= !0x20; // ビット5を0に設定（P15選択）
            ret = (ret & 0xF0) | self.key_state.get_action();
        }

        ret
    }

    pub fn write(&mut self, value: u8) {
        self.direction_selected = value & 0x10 == 0;
        self.action_selected = value & 0x20 == 0;
    }

    pub fn set_key(&mut self, context: &mut impl Context, key_state: JoypadKeyState) {
        let prev_key = self.key_state.0.bits();
        let cur_key = key_state.0.bits();

        let changed_keys = prev_key ^ cur_key;
        let pressed_keys = changed_keys & !cur_key;

        if pressed_keys != 0 {
            context.set_interrupt_joypad(true);
        }

        self.key_state = key_state;
    }
}

pub enum JoypadKey {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

#[derive(Clone, Copy)]
pub struct JoypadKeyState(Keys);

impl JoypadKeyState {
    pub fn new() -> Self {
        Self(Keys::empty())
    }

    pub fn set_key(&mut self, key: JoypadKey, pressed: bool) {
        let key_flag = match key {
            JoypadKey::Right => Keys::RIGHT,
            JoypadKey::Left => Keys::LEFT,
            JoypadKey::Up => Keys::UP,
            JoypadKey::Down => Keys::DOWN,
            JoypadKey::A => Keys::A,
            JoypadKey::B => Keys::B,
            JoypadKey::Select => Keys::SELECT,
            JoypadKey::Start => Keys::START,
        };

        if pressed {
            self.0.insert(key_flag);
        } else {
            self.0.remove(key_flag);
        }
    }

    fn get_direction(&self) -> u8 {
        (!self.0.bits()) & 0x0F
    }

    fn get_action(&self) -> u8 {
        ((!self.0.bits()) >> 4) & 0x0F
    }
}
