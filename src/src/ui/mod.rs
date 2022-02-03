pub mod move_z;

/*
use drivers::stepper::prelude::*;

use core::fmt::Write;
use cstr_core::CString;

use lvgl::*;
use lvgl::style::*;
use lvgl::widgets::*;


fn draw() {


    let mut screen_style = Style::default();
    screen_style.set_bg_color(State::DEFAULT, Color::from_rgb((80, 80, 80)));
    //screen_style.set_radius(State::DEFAULT, 0);
    //screen.add_style(Part::Main, screen_style).unwrap();
    //screen_style.fl

    let spacing = 12;

    {
        let mut label = Label::new(&mut screen).unwrap();
        label.set_text(CString::new("Turbo Resin v0.1.1").unwrap().as_c_str()).unwrap();
        label.set_align(&mut screen, Align::InBottomRight, -5, -5).unwrap();
    }

    let mut btn_up = {
        let mut btn = Btn::new(&mut screen).unwrap();
        btn.set_align(&mut screen, Align::InTopMid, 0, 2*spacing).unwrap();
        let mut label = Label::new(&mut btn).unwrap();
        label.set_text(CString::new("Move Up").unwrap().as_c_str()).unwrap();
        label.set_label_align(LabelAlign::Center).unwrap();
        btn.set_checkable(true).unwrap();
        btn
    };

    let mut btn_up_active = false;
    let mut btn_down_active = false;

    let mut btn_down = {
        let mut btn = Btn::new(&mut screen).unwrap();
        btn.set_align(&mut btn_up, Align::OutBottomMid, 0, spacing).unwrap();
        let mut label = Label::new(&mut btn).unwrap();
        label.set_text(CString::new("Move Down").unwrap().as_c_str()).unwrap();
        label.set_label_align(LabelAlign::Center).unwrap();
        btn.set_checkable(true).unwrap();
        btn
    };

    btn_up.on_event(|_, event| {
        if let lvgl::Event::Clicked = event {
            btn_up_active = !btn_up_active;
            if btn_up_active {
                stepper.modify(|s| s.set_target_relative(100.0.mm()));
                unsafe {
                    lvgl_sys::lv_btn_set_state(btn_down.raw().unwrap().as_mut(),
                        lvgl_sys::LV_BTN_STATE_DISABLED as u8);
                }
            } else {
                stepper.modify(|s| s.controlled_stop());
            }
        }
    }).unwrap();

    btn_down.on_event(|_, event| {
        if let lvgl::Event::Clicked = event {
            btn_down_active = !btn_down_active;
            if btn_down_active {
                stepper.modify(|s| s.set_target_relative((-100.0).mm()));
                unsafe {
                    lvgl_sys::lv_btn_set_state(btn_up.raw().unwrap().as_mut(),
                        lvgl_sys::LV_BTN_STATE_DISABLED as u8);
                }
            } else {
                stepper.modify(|s| s.controlled_stop());
            }
        }
    }).unwrap();

    let (speed_slider, mut speed_label) = {
        let mut speed_slider = Slider::new(&mut screen).unwrap();
        speed_slider.set_align(&mut btn_down, Align::OutBottomMid, 0, 2*spacing).unwrap();

        let mut speed_label = Label::new(&mut screen).unwrap();
        speed_label.set_align(&mut speed_slider, Align::OutBottomLeft, 70, spacing).unwrap();
        speed_label.set_width(320).unwrap();
        speed_label.set_text(&CString::new("Max Speed: 5 mm/s").unwrap()).unwrap();
        speed_label.set_label_align(LabelAlign::Left).unwrap();

        let value = unsafe { lvgl_sys::lv_bar_set_range(speed_slider.raw().unwrap().as_mut(), 1500, 10_000) };
        unsafe { lvgl_sys::lv_bar_set_value(speed_slider.raw().unwrap().as_mut(), 10_000, 0) };
        speed_slider.on_event(|slider, event| {
            let value = unsafe { lvgl_sys::lv_slider_get_value(slider.raw().unwrap().as_ref()) };
            let value = (value as f32)/10000.0;
            let value = value*value*value;

            let value = value * 30.0;

            stepper.modify(|s| s.set_max_speed(Some(value.mm())));
        }).unwrap();

        (speed_slider, speed_label)
    };

    let mut position_label = {
        let mut position = Label::new(&mut screen).unwrap();
        position.set_align(&mut speed_label, Align::OutBottomLeft, 0, 0).unwrap();
        position.set_text(CString::new("Position: 0 mm").unwrap().as_c_str()).unwrap();
        position
    };

    loop {
        let text = {
            let mut string = arrayvec::ArrayString::<100>::new();
            let p = stepper.access(|s| s.current_position);
            let _ = write!(&mut string, "Position: {:.2} mm", p.as_mm());
            CString::new(&*string).unwrap()
        };
        position_label.set_text(&text).unwrap();

        let text = {
            let mut string = arrayvec::ArrayString::<100>::new();
            let p = stepper.access(|s| s.max_speed);
            let _ = write!(&mut string, "Max Speed: {:.2} mm/s", p.as_mm());
            CString::new(&*string).unwrap()
        };
        speed_label.set_text(&text).unwrap();

        ui.task_handler();

        if stepper.access(|s| s.is_idle()) {
            btn_down_active = false;
            btn_up_active = false;

            // turn off warning. Weird.
            if btn_down_active || btn_up_active {}

            unsafe {
                lvgl_sys::lv_btn_set_state(btn_up.raw().unwrap().as_mut(),
                    lvgl_sys::LV_BTN_STATE_RELEASED as u8);
                lvgl_sys::lv_btn_set_state(btn_down.raw().unwrap().as_mut(),
                    lvgl_sys::LV_BTN_STATE_RELEASED as u8);
            }
        }

        ui.tick_inc(Duration::from_millis(20));
    }

}

*/
