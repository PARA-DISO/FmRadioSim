use iced::time;
use iced::{
    executor,
    widget::{Column, Container, Text},
    window, Alignment, Application, Command, Degrees, Element, Font, Length, Point, Rectangle,
    Renderer, Settings, Subscription, Theme, Vector,
};
use plotters::{coord::Shift, prelude::*};
use plotters_backend::DrawingBackend;
use plotters_iced::{plotters_backend, Chart, ChartWidget, DrawingArea};
use spectrum_analyzer::scaling::divide_by_N_sqrt;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
const TITLE_FONT_SIZE: u16 = 22;
mod fm_modulator;
mod lpf;
use lpf::Lpf;
mod composite;
fn main() {
    State::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}

#[derive(Debug)]
enum Message {
    Tick,
}

struct State {
    chart: MyChart,
}

impl Application for State {
    type Message = self::Message;
    type Executor = executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                chart: MyChart::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Split Chart Example".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Tick => {
                self.chart.next();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content = Column::new()
            .spacing(20)
            .align_items(Alignment::Start)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(Text::new("Iced test chart").size(TITLE_FONT_SIZE))
            .push(self.chart.view());

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // window::frames().map(|_| Message::Tick)
        time::every(time::Duration::from_millis(50)).map(|_| Message::Tick)
    }
}
// use std::collections::VecDeque;
const SIZE: usize = 512;
const AUDIO_SAMPLE_RATE: usize = 44100;
const SAMPLE_RATE: usize = 500_000 * 4;
const SIGNAL_FREQ: f64 = 440_f64;
const CARRIER_FREQ: f64 = 500_000f64;

use fm_modulator::{FmDeModulator, FmModulator};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
struct MyChart {
    t: f64,
    sig: Vec<f32>,
    carrier: Vec<f32>,
    modulator: FmModulator,
    demodulator: FmDeModulator,
    up_sampler: FastFixedIn<f32>,
}
impl MyChart {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            sig: vec![0f32; SIZE],
            carrier: vec![0f32; SIZE],
            modulator: FmModulator::from(CARRIER_FREQ, SAMPLE_RATE as f64),
            demodulator: FmDeModulator::from(CARRIER_FREQ, SAMPLE_RATE as f64),
            up_sampler: FastFixedIn::new(
                SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
                SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
                PolynomialDegree::Linear,
                SIZE,
                1,
            )
            .unwrap(),
        }
    }

    fn view(&self) -> Element<Message> {
        let chart = ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fill);

        chart.into()
    }
    fn next(&mut self) {
        // 信号の作成
        for i in 0..SIZE {
            // self.sig[i] = ((self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin() as f32 + (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ * 2.).sin() as f32);
            self.sig[i] = (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin() as f32;
            // self.carrier[i] = ((self.t * 2f64 * std::f64::consts::PI * CARRIER_FREQ).cos() as f32);
            self.t += 1f64 / AUDIO_SAMPLE_RATE as f64;
        }
        // upsample
        // dbg!(self.up_sampler.output_frames_next());
        let s = self
            .up_sampler
            .process(&[self.sig.as_slice()], None)
            .unwrap();
        // 変調
        let modulated = self.modulator.modulate(&s[0]);
        // 復調
        self.demodulator.demodulate(modulated);
        // dbg!(&self.sig);
        // unreachable!();
    }
}
use rustfft::num_complex::ComplexFloat;
impl Chart<Message> for MyChart {
    type State = ();
    // leave it empty
    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, _builder: ChartBuilder<DB>) {}

    fn draw_chart<DB: DrawingBackend>(&self, _state: &Self::State, root: DrawingArea<DB, Shift>) {
        // let  serufu = unsafe {(self as *const Self).cast_mut().as_mut().unwrap()};
        let children = root.split_evenly((3, 4));
        let labels: [&str; 12] = [
            "input",
            "input spectrum",
            "carrier",
            "carrier spectrum",
            "modulated",
            "modulated spectrum",
            "lpf",
            "",
            "demodulate",
            "demodulated spectrum",
            "",
            "",
        ];
        let modurated_buffer = self.modulator.get_buffer();
        let demodulate = self.demodulator.get_buffer();
        for (i, area) in children.iter().enumerate() {
            let builder = ChartBuilder::on(area);
            match i {
                0 => draw_chart(builder, labels[0], &self.sig, AUDIO_SAMPLE_RATE),
                1 => draw_spectrum(builder, labels[1], &self.sig, AUDIO_SAMPLE_RATE),
                2 => {
                    if !modurated_buffer.is_empty() {
                        draw_chart(builder, labels[4], modurated_buffer, SAMPLE_RATE)
                    }
                }
                3 => draw_chart(builder, labels[8], demodulate, SAMPLE_RATE),

                _ => {}
            }
        }
    }
}
fn draw_chart<DB: DrawingBackend>(
    mut chart: ChartBuilder<DB>,
    label: &str,
    data: &[f32],
    sample_rate: usize,
) {
    let mut chart = chart
        .margin(30)
        .caption(label, ("sans-serif", 22))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..data.len() as f32 / sample_rate as f32, -1f32..1f32)
        .unwrap();

    chart
        .configure_mesh()
        .x_labels(3)
        .y_labels(3)
        // .y_label_style(
        //     ("sans-serif", 15)
        //         .into_font()
        //         .color(&plotters::style::colors::BLACK.mix(0.8))
        //         .transform(FontTransform::RotateAngle(30.0)),
        // )
        .draw()
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            data.iter()
                // .take(SIZE)
                .enumerate()
                .map(|(i, x)| (i as f32 / sample_rate as f32, *x)),
            // (-50..=50)
            //     .map(|x| x as f32 / 50.0)
            //     .map(|x| (x, x.powf(power as f32))),
            &RED,
        ))
        .unwrap();
}

fn draw_spectrum<DB: DrawingBackend>(
    mut chart: ChartBuilder<DB>,
    label: &str,
    data: &[f32],
    sample_rate: usize,
) {
    let spectrum = samples_fft_to_spectrum(
        data,
        sample_rate as u32,
        FrequencyLimit::All,
        Some(&divide_by_N_sqrt),
    )
    .unwrap();
    let mut chart = chart
        .margin(30)
        .caption(
            format!("{} ({}Hz)", label, spectrum.max().0),
            ("sans-serif", 22),
        )
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..sample_rate as f32 / 2f32, -1f32..3f32)
        .unwrap();

    chart
        .configure_mesh()
        .x_labels(3)
        .y_labels(3)
        // .y_label_style(
        //     ("sans-serif", 15)
        //         .into_font()
        //         .color(&plotters::style::colors::BLACK.mix(0.8))
        //         .transform(FontTransform::RotateAngle(30.0)),
        // )
        .draw()
        .unwrap();
    chart
        .draw_series(LineSeries::new(
            spectrum.data().iter().map(|(f, x)| (f.val(), x.val())),
            // (-50..=50)
            //     .map(|x| x as f32 / 50.0)
            //     .map(|x| (x, x.powf(power as f32))),
            &RED,
        ))
        .unwrap();
}
