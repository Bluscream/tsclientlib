use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use tsproto::license::Licenses;
use tsproto_types::crypto::EccKeyPubEd25519;

fn license(b: &mut Bencher, license: &[u8]) {
	let licenses = Licenses::parse_ignore_expired(license).unwrap();

	b.iter(|| {
		let derived_key =
			licenses.derive_public_key(EccKeyPubEd25519::from_bytes(tsproto::ROOT_KEY)).unwrap();
		derived_key.compress().0
	});
}

fn standard_license(b: &mut Bencher) {
	license(b, &base64::decode("AQA1hUFJiiSs0wFXkYuPUJVcDa6XCrZTcsvkB0\
		Ffzz4CmwIITRXgCqeTYAcAAAAgQW5vbnltb3VzAACiIBip9hQaK6P3QhwOJs/BkPn0i\
		oyIDPaNgzJ6M8x0kiAJf4hxCYAxMQ==").unwrap());
}

fn aal_license(b: &mut Bencher) {
	license(b, &base64::decode("AQCvbHFTQDY/terPeilrp/ECU9xCH5U3xC92lY\
		TNaY/0KQAJFueAazbsgAAAACVUZWFtU3BlYWsgU3lzdGVtcyBHbWJIAABhl9gwla/UJ\
		p2Eszst9TRVXO/PeE6a6d+CTI6Pg7OEVgAJc5CrL4Nh8gAAACRUZWFtU3BlYWsgc3lz\
		dGVtcyBHbWJIAACvTQIgpv6zmLZq3znh7ygmOSokGFkFjz4bTigrOnetrgIJdIIACdS\
		/gAYAAAAAU29zc2VuU3lzdGVtcy5iaWQAADY7+uV1CQ1niOvYSdGzsu83kPTNWijovr\
		3B78eHGeePIAm98vQJvpu0").unwrap());
}

fn bench_license(c: &mut Criterion) {
	c.bench_function("standard license", standard_license);
	c.bench_function("aal license", aal_license);
}

criterion_group!(benches, bench_license);
criterion_main!(benches);
