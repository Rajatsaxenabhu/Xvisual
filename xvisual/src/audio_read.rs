use pipewire as pw;
use pw::{properties::properties, spa};
use ringbuf::traits::Producer;
use spa::param::format::{MediaSubtype, MediaType};
use spa::param::format_utils;
use spa::pod::Pod;

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
    channels: u32,
}


pub fn capture_audio(
    mut prod: impl Producer<Item = f32> + Send + 'static,
) -> Result<(), pw::Error> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let data = UserData {
        format: Default::default(),
        channels: 0,
    };

    let props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::STREAM_CAPTURE_SINK => "true",
        *pw::keys::NODE_LATENCY => "512/48000",
    };

    let stream = pw::stream::StreamBox::new(&core, "audio-capture", props)?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };

            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }

            let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                return;
            };

            if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                return;
            }

            if user_data.format.parse(param).is_ok() {
                user_data.channels = user_data.format.channels();
            }
        })
        .process(move |stream, user_data| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let data = &mut datas[0];

            let Some(samples) = data.data() else {
                return;
            };

            let channels = user_data.channels;
            if channels == 0 {
                return;
            };

            let n_f32 = samples.len() / 4;
            let frames = n_f32 / channels as usize;

            for i in 0..frames {
                let mut sum = 0.0;

                for ch in 0..channels {
                    let idx = (i * channels as usize + ch as usize) * 4;

                    let mut bytes = [0u8; 4];
                    bytes.copy_from_slice(&samples[idx..idx + 4]);

                    let sample = f32::from_le_bytes(bytes);
                    sum += sample;
                }

                let mono = sum / channels as f32;

                // IMPORTANT: avoid blocking RT thread
                let _ = prod.try_push(mono);
            }
        })
        .register()?;

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);

    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };

    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    mainloop.run();
    Ok(())
}
