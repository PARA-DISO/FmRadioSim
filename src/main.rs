use std::f32::INFINITY;

use iced::time;

use iced::{
    executor,
    widget::{Column, Container, Text}, Alignment, Application, Command, Element, Length, Settings, Subscription, Theme,
};
use plotters::{coord::Shift, prelude::*};
use plotters_iced::{Chart, ChartWidget,};
use spectrum_analyzer::scaling::{
    scale_to_zero_to_one,
};
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
// const SIGNAL_FREQ: f64 = 440_f64;
const SIGNAL_FREQ: f64 = 7_000_f64;
const CARRIER_FREQ: f64 = 1_000_000f64;
const CUT_OFF: f64 = 200_000.;
const NOISE: f32 = -70.;
const A:f64 = 0.5;
use fm_modulator::{FmDeModulator, FmModulator};

use composite::{CompositeSignal, RestoredSignal};
use rubato::{FastFixedOut, PolynomialDegree, Resampler};
struct MyChart {
    t: f64,
    lr: [Vec<f32>; 2],
    sig: Vec<f32>,
    carrier: Vec<f32>,
    modulator: FmModulator,
    demodulator: FmDeModulator,
    composite: CompositeSignal,
    restor: RestoredSignal,
    // up_sampler: FastFixedIn<f32>,
    up_sampler: FastFixedOut<f32>,
    continue_flag: bool,
    transmission_line: transmission_line::TransmissionLine,
}
impl MyChart {
    pub fn new() -> Self {
        let up_sampler = FastFixedOut::new(
            SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
            SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
            PolynomialDegree::Linear,
            BUFFER_SIZE,
            2,
        )
        .unwrap();
        let buffer_size = dbg!(up_sampler.input_frames_next());
        let composite = CompositeSignal::new(SAMPLE_RATE as f32);

        let restor = RestoredSignal::new(SAMPLE_RATE as f32);
        Self {
            t: 0.0,
            sig: vec![0f32; BUFFER_SIZE],
            lr: [vec![0.; buffer_size], vec![0.; buffer_size]],
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
                // self.lr[0][i] =
                //     ((self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin() * A )
                //         as f32;
                // self.lr[1][i] =
                //     ((self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ * 2.)
                //         .sin()  * A) as f32;
                self.t += 1f64 / AUDIO_SAMPLE_RATE as f64;
            }
            // upsample
            let lr = self
                .up_sampler
                .process(&[&self.lr[0], &self.lr[1]], None)
                .unwrap();
            self.composite.process(&lr[0], &lr[1]);

            // 変調
            let modulated =
                self.modulator.modulate(self.composite.get_buffer());
            self.transmission_line
                .process_to_buf(&mut self.sig, modulated);
            // 復調
            self.demodulator.demodulate(&self.sig);
            // コンポジット
            self.restor.process(self.demodulator.get_buffer());
            // self.restor.process(self.composite.get_buffer())
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
                // 2 => draw_chart(
                //     builder,
                //     labels[2],
                //     &(self.lr[0])
                //         .iter()
                //         .zip((self.lr[1]).iter())
                //         .map(|(&l, &r)| l + r)
                //         .collect::<Vec<f32>>(),
                //     AUDIO_SAMPLE_RATE,
                // ),
                // 3 => draw_chart(
                //     builder,
                //     labels[3],
                //     &(self.lr[0])
                //         .iter()
                //         .zip((self.lr[1]).iter())
                //         .map(|(&l, &r)| l - r)
                //         .collect::<Vec<f32>>(),
                //     AUDIO_SAMPLE_RATE,
                // ),
                2 => draw_chart(
                    builder,
                    labels[4],
                    self.composite.get_buffer(),
                    self.composite.sample_rate() as usize,
                ),
                3 => {
                    if !self.composite.get_buffer().is_empty() {
                        draw_spectrum(
                            builder,
                            labels[5],
                            self.composite.get_buffer(),
                            CompositeSignal::DEFAULT_SAMPLE_RATE as usize,
                            // FrequencyLimit::Max(CompositeSignal::DEFAULT_SAMPLE_RATE)
                            FrequencyLimit::All,
                        )
                    }
                }
                4 => draw_chart(
                    builder,
                    labels[6],
                    modurated_buffer,
                    SAMPLE_RATE,
                ),
                5 => {
                    if !modurated_buffer.is_empty() {
                        draw_spectrum(
                            builder,
                            labels[7],
                            modurated_buffer,
                            SAMPLE_RATE,
                            FrequencyLimit::All,
                        );
                    }
                }
                6 => draw_chart(
                    builder,
                    "transmission line",
                    &self.sig,
                    SAMPLE_RATE,
                ),
                7 => draw_spectrum(
                    builder,
                    "transmission spectrum",
                    &self.sig,
                    SAMPLE_RATE,
                    FrequencyLimit::All,
                ),
                8 => draw_chart(builder, labels[8], demodulate, SAMPLE_RATE),
                9 => {
                    if !demodulate.is_empty() {
                        draw_spectrum(
                            builder,
                            labels[9],
                            demodulate,
                            SAMPLE_RATE,
                            /*FrequencyLimit::Max(CompositeSignal::DEFAULT_SAMPLE_RATE)*/
                            FrequencyLimit::All,
                        );
                    }
                }
                10 => draw_chart(
                    builder,
                    labels[10],
                    &restore_buffer[0],
                    SAMPLE_RATE,
                ),
                11 => draw_chart(
                    builder,
                    labels[11],
                    &restore_buffer[1],
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
            -1f32..1f32,
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
