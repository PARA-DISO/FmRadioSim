use std::f32::INFINITY;

use iced::time;

use iced::{
    executor,
    widget::{Column, Container, Text},
    Alignment, Application, Command, Element, Length, Settings, Subscription,
    Theme,
};
use plotters::{coord::Shift, prelude::*};
use plotters_iced::{Chart, ChartWidget};
use spectrum_analyzer::scaling::scale_to_zero_to_one;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
const TITLE_FONT_SIZE: u16 = 22;
mod filter;
mod fm_modulator;

pub mod composite;
mod transmission_line;
fn main() {
    State::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}
mod utils {

    // pub fn downsample_f32(dst: &mut [f32], x: &[f32], factor: f64) {
    //     let x_len = x.len() as f64;
    //     // let mut dst = Vec::with_capacity((x_len / factor + 0.5) as usize);
    //     let mut i = 0.;
    //     let mut j = 0;
    //     let max = x.len() - 1;
    //     while (i + 0.5) < x_len && j < dst.len() {
    //         let idx_a = i.floor();
    //         let idx_b = i.ceil() as usize;
    //         let p = i - idx_a;
    //         let q = 1. - p;
    //         dst[j] = x[(idx_a as usize).min(max)] * q as f32
    //             + x[idx_b.min(max)] * p as f32;
    //         i += factor;
    //         j += 1;
    //     }
    // }
    pub fn downsample_f32(x: &[f32], factor: f64) -> Vec<f32> {
        let x_len = x.len() as f64;
        let mut dst = Vec::with_capacity((x_len / factor + 0.5) as usize);
        let mut i = 0.;
        let max = x.len() - 1;
        while (i + 0.5) < x_len {
            let idx_a = i.floor();
            let idx_b = i.ceil() as usize;
            let p = i - idx_a;
            let q = 1. - p;
            dst.push(
                x[(idx_a as usize).min(max)] * q as f32
                    + x[idx_b.min(max)] * p as f32,
            );
            i += factor;
        }
        dst
    }
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
            .push(Text::new("FM Stereo").size(TITLE_FONT_SIZE))
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
const BUFFER_SIZE: usize = 512 << 4;
const AUDIO_SAMPLE_RATE: usize = 44100;
const SAMPLE_RATE: usize = 1_000_000 * 4;
const SIGNAL_FREQ: f64 = 440_f64;
const CARRIER_FREQ: f64 = 1_000_000f64;
const CUT_OFF: f64 = 200_000.;
const NOISE: f32 = -INFINITY;
use fm_modulator::{FmDeModulator, FmModulator};

use composite::{CompositeSignal, RestoredSignal};
use rubato::{FastFixedIn, FastFixedOut, PolynomialDegree, Resampler};
struct MyChart {
    t: f64,
    lr: [Vec<f32>; 2],
    buffer: [Vec<f32>; 2],
    // sig: Vec<f32>,
    carrier: Vec<f32>,
    modulator: FmModulator,
    demodulator: FmDeModulator,
    composite: CompositeSignal,
    restor: RestoredSignal,
    // up_sampler: FastFixedIn<f32>,
    up_sampler: FastFixedIn<f32>,
    continue_flag: bool,
    transmission_line: transmission_line::TransmissionLine,
}
impl MyChart {
    pub fn new() -> Self {
        let up_sampler = FastFixedIn::new(
            SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
            SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
            PolynomialDegree::Linear,
            SIZE,
            2,
        )
        .unwrap();
        let buffer_size = dbg!(up_sampler.output_frames_next());
        let composite = CompositeSignal::new(SAMPLE_RATE as f32);

        let restor = RestoredSignal::new(SAMPLE_RATE as f32);
        Self {
            t: 0.0,
            // sig: vec![0f32; BUFFER_SIZE],
            lr: [vec![0.; SIZE], vec![0.; SIZE]],
            buffer: [vec![0.; buffer_size], vec![0.; buffer_size]],
            carrier: Vec::new(), //vec![0f32; SIZE],
            modulator: FmModulator::from(CARRIER_FREQ, SAMPLE_RATE as f64),
            demodulator: FmDeModulator::from(
                CARRIER_FREQ,
                SAMPLE_RATE as f64,
                CARRIER_FREQ + CUT_OFF,
            ),
            composite,
            restor,
            up_sampler,
            transmission_line: transmission_line::TransmissionLine::from_snr(
                NOISE,
            ),
            continue_flag: true,
        }
    }

    fn view(&self) -> Element<Message> {
        let chart = ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fill);

        chart.into()
    }
    fn next(&mut self) {
        if self.continue_flag {
            // 信号の作成
            for i in 0..self.lr[0].len() {
                self.lr[0][i] =
                    (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin()
                        as f32;
                self.lr[1][i] =
                    (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ * 2.)
                        .sin() as f32;
                self.t += 1f64 / AUDIO_SAMPLE_RATE as f64;
            }
            // // upsample
            // let _ = self
            //     .up_sampler
            //     .process_into_buffer(&[&self.lr[0], &self.lr[1]],&mut self.buffer.as_slice(), None)
            //     .unwrap();
            // self.composite.process_to_buffer(&self.buffer[0], &self.buffer[1], &mut self.buffer[0]);

            // // 変調
            // let modulated =
            //     self.modulator.modulate(self.composite.get_buffer());
            // self.transmission_line
            //     .process_to_buf(&mut self.sig, modulated);
            // // 復調
            // self.demodulator.demodulate(&self.sig);
            // // コンポジット
            // self.restor.process(self.demodulator.get_buffer());

            let buf_size = (self.up_sampler.output_frames_next());
            let mut buf1 = unsafe {
                std::slice::from_raw_parts_mut(
                    self.buffer[0].as_ptr().cast_mut(),
                    buf_size,
                )
            };

            let mut buf2 = unsafe {
                std::slice::from_raw_parts_mut(
                    self.buffer[1].as_ptr().cast_mut(),
                    buf_size,
                )
            };
            let buffer = self
                .up_sampler
                .process_into_buffer(
                    &[&self.lr[0], &self.lr[1]],
                    &mut [&mut buf1, &mut buf2],
                    None,
                )
                .unwrap();
            self.composite.process_to_buffer(
                self.buffer[0].as_slice(),
                self.buffer[1].as_slice(),
                buf1,
            );
            self.modulator
                .modulate_to_buffer(self.buffer[0].as_slice(), buf2);
            self.demodulator
                .demodulate_to_buffer(self.buffer[1].as_slice(), buf1);
            self.restor.process_to_buffer(
                self.buffer[0].as_slice(),
                buf1,
                buf2,
            );

            let factor = buf_size as f64 / self.lr[0].len() as f64;
            // let samples = self.lr.as_mut();
            self.lr[0]
                .iter_mut()
                .zip(
                    utils::downsample_f32(
                        // self.lr[1].as_mut_slice(),
                        &self.buffer[0],
                        factor,
                    )
                    .iter(),
                )
                .for_each(|(d, s)| *d = *s);
            self.lr[1]
                .iter_mut()
                .zip(
                    utils::downsample_f32(
                        // self.lr[1].as_mut_slice(),
                        &self.buffer[1],
                        factor,
                    )
                    .iter(),
                )
                .for_each(|(d, s)| *d = *s);
        }
    }
}

impl Chart<Message> for MyChart {
    type State = ();
    // leave it empty
    fn build_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        _builder: ChartBuilder<DB>,
    ) {
    }

    fn draw_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        root: DrawingArea<DB, Shift>,
    ) {
        let children = root.split_evenly((3, 4));

        let labels: [&str; 12] = [
            "L In",
            "R In",
            "L+R",
            "L-R",
            "Composite",
            "Composite Spectrum",
            "FM Modulated",
            "Modulated Spectrum",
            "FM Demodulated",
            "Demodulated Spectrum",
            "L Out",
            "R Out",
        ];
        let modurated_buffer = self.modulator.get_buffer();
        let demodulate = self.demodulator.get_buffer();
        let restore_buffer = self.restor.get_buffer();
        for (i, area) in children.iter().enumerate() {
            let builder = ChartBuilder::on(area);
            match i {
                0 => draw_chart(
                    builder,
                    labels[0],
                    &self.lr[0],
                    AUDIO_SAMPLE_RATE,
                ),
                1 => draw_chart(
                    builder,
                    labels[1],
                    &self.lr[1],
                    AUDIO_SAMPLE_RATE,
                ),
                // // 2 => draw_chart(
                // //     builder,
                // //     labels[2],
                // //     &(self.lr[0])
                // //         .iter()
                // //         .zip((self.lr[1]).iter())
                // //         .map(|(&l, &r)| l + r)
                // //         .collect::<Vec<f32>>(),
                // //     AUDIO_SAMPLE_RATE,
                // // ),
                // // 3 => draw_chart(
                // //     builder,
                // //     labels[3],
                // //     &(self.lr[0])
                // //         .iter()
                // //         .zip((self.lr[1]).iter())
                // //         .map(|(&l, &r)| l - r)
                // //         .collect::<Vec<f32>>(),
                // //     AUDIO_SAMPLE_RATE,
                // // ),
                // 2 => draw_chart(
                //     builder,
                //     labels[4],
                //     self.composite.get_buffer(),
                //     self.composite.sample_rate() as usize,
                // ),
                // 3 => {
                //     if !self.composite.get_buffer().is_empty() {
                //         draw_spectrum(
                //             builder,
                //             labels[5],
                //             self.composite.get_buffer(),
                //             CompositeSignal::DEFAULT_SAMPLE_RATE as usize,
                //             // FrequencyLimit::Max(CompositeSignal::DEFAULT_SAMPLE_RATE)
                //             FrequencyLimit::All,
                //         )
                //     }
                // }
                // 4 => draw_chart(
                //     builder,
                //     labels[6],
                //     modurated_buffer,
                //     SAMPLE_RATE,
                // ),
                // 5 => {
                //     if !modurated_buffer.is_empty() {
                //         draw_spectrum(
                //             builder,
                //             labels[7],
                //             modurated_buffer,
                //             SAMPLE_RATE,
                //             FrequencyLimit::All,
                //         );
                //     }
                // }
                // 6 => draw_chart(
                //     builder,
                //     "transmission line",
                //     &self.sig,
                //     SAMPLE_RATE,
                // ),
                // 7 => draw_spectrum(
                //     builder,
                //     "transmission spectrum",
                //     &self.sig,
                //     SAMPLE_RATE,
                //     FrequencyLimit::All,
                // ),
                // 8 => draw_chart(builder, labels[8], demodulate, SAMPLE_RATE),
                // 9 => {
                //     if !demodulate.is_empty() {
                //         draw_spectrum(
                //             builder,
                //             labels[9],
                //             demodulate,
                //             SAMPLE_RATE,
                //             /*FrequencyLimit::Max(CompositeSignal::DEFAULT_SAMPLE_RATE)*/
                //             FrequencyLimit::All,
                //         );
                //     }
                // }
                10 => draw_chart(
                    builder,
                    labels[10],
                    &self.buffer[0],
                    SAMPLE_RATE,
                ),
                11 => draw_chart(
                    builder,
                    labels[11],
                    &self.buffer[1],
                    SAMPLE_RATE,
                ),
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
        .margin(10)
        .caption(label, ("sans-serif", 22))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            0f32..data.len() as f32 / sample_rate as f32,
            -3f32..3f32,
        )
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
    limit: FrequencyLimit,
) {
    let spectrum = samples_fft_to_spectrum(
        data,
        sample_rate as u32,
        limit,
        Some(&scale_to_zero_to_one),
    )
    .unwrap();
    let mut chart = chart
        .margin(10)
        .caption(
            format!("{} ({}Hz)", label, spectrum.max().0),
            ("sans-serif", 22),
        )
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..sample_rate as f32 / 2f32, 0f32..1f32)
        .unwrap();

    chart
        .configure_mesh()
        .x_labels(5)
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
