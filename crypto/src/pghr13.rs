use bn::pairing;
pub use bn::{
    arith::{U256, U512},
    AffineG1, AffineG2, CurveError, Fq, Fq2, Fr, Group, G1, G2,
};
use json::pghr13 as json;

#[derive(Clone)]
pub struct VerifyingKey {
    pub a: G2,
    pub b: G1,
    pub c: G2,
    pub z: G2,
    pub gamma: G2,
    pub gamma_beta_1: G1,
    pub gamma_beta_2: G2,
    pub ic: Vec<G1>,
}

impl From<json::VerifyingKey> for VerifyingKey {
    fn from(v: json::VerifyingKey) -> Self {
        VerifyingKey {
            a: v.a.into(),
            b: v.b.into(),
            c: v.c.into(),
            z: v.z.into(),
            gamma: v.gamma.into(),
            gamma_beta_1: v.gamma_beta_1.into(),
            gamma_beta_2: v.gamma_beta_2.into(),
            ic: v.ic.into_iter().map(Into::into).collect(),
        }
    }
}

impl ::std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "[Verifying Key: TODO]")
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidFieldElement,
    InvalidCurvePoint,
    InvalidRawInput,
    InvalidU256Encoding,
    InvalidU512Encoding,
    NotFqMember,
    NotFq2Member,
    InvalidSignPrefix,
    Curve(CurveError),
}

impl From<CurveError> for Error {
    fn from(e: CurveError) -> Self {
        Error::Curve(e)
    }
}

#[derive(Clone)]
pub struct Proof {
    pub a: G1,
    pub a_prime: G1,
    pub b: G2,
    pub b_prime: G1,
    pub c: G1,
    pub c_prime: G1,
    pub k: G1,
    pub h: G1,
}

impl Proof {
    pub fn from_raw(data: &[u8; 296]) -> Result<Self, Error> {
        Ok(Proof {
            a: G1::from_compressed(&data[0..33])?,
            a_prime: G1::from_compressed(&data[33..66])?,
            b: G2::from_compressed(&data[66..131])?,
            b_prime: G1::from_compressed(&data[131..164])?,
            c: G1::from_compressed(&data[164..197])?,
            c_prime: G1::from_compressed(&data[197..230])?,
            k: G1::from_compressed(&data[230..263])?,
            h: G1::from_compressed(&data[263..296])?,
        })
    }
}

pub fn verify(vk: &VerifyingKey, primary_input: &[Fr], proof: &Proof) -> bool {
    let p2 = G2::one();

    // 1. compute accumulated input circuit (evaluate the polynomial)
    let mut acc = G1::zero();
    for (&x, &ic) in primary_input.iter().zip(vk.ic[1..].iter()) {
        acc = acc + (ic * x);
    }
    acc = acc + vk.ic[0];

    // 2. check validity of knowledge commitments for A, B, C:
    pairing(proof.a, vk.a) == pairing(proof.a_prime, p2) &&
	pairing(vk.b, proof.b) == pairing(proof.b_prime, p2) &&
	pairing(proof.c, vk.c) == pairing(proof.c_prime, p2) &&

	// 3. check same coefficients were used:
	pairing(proof.k, vk.gamma) ==
		pairing(acc + proof.a + proof.c, vk.gamma_beta_2) * pairing(vk.gamma_beta_1, proof.b) &&
		// 4. check QAP divisibility
		pairing(acc + proof.a, proof.b) == pairing(proof.h, vk.z) * pairing(proof.c, p2)
}

#[cfg(test)]
mod tests {

    use super::*;
    use json;

    fn vkey() -> VerifyingKey {
        json::pghr13::decode(include_bytes!("../../res/sprout-verifying-key.json"))
            .expect("known to be good")
            .into()
    }

    fn pgh13_proof(hex: &'static str) -> [u8; 296] {
        use hex::FromHex;

        assert_eq!(hex.len(), 296 * 2);

        let bytes: Vec<u8> = hex.from_hex().expect("is static and should be good");
        let mut arr = [0u8; 296];
        arr[..].copy_from_slice(&bytes[..]);

        arr
    }

    fn sample_pghr_proof() -> [u8; 296] {
        pgh13_proof("022cbbb59465c880f50d42d0d49d6422197b5f823c2b3ffdb341869b98ed2eb2fd031b271702bda61ff885788363a7cf980a134c09a24c9911dc94cbe970bd613b700b0891fe8b8b05d9d2e7e51df9d6959bdf0a3f2310164afb197a229486a0e8e3808d76c75662b568839ebac7fbf740db9d576523282e6cdd1adf8b0f9c183ae95b0301fa1146d35af869cc47c51cfd827b7efceeca3c55884f54a68e38ee7682b5d102131b9b1198ed371e7e3da9f5a8b9ad394ab5a29f67a1d9b6ca1b8449862c69a5022e5d671e6989d33c182e0a6bbbe4a9da491dbd93ca3c01490c8f74a780479c7c031fb473670cacde779713dcd8cbdad802b8d418e007335919837becf46a3b1d0e02120af9d926bed2b28ed8a2b8307b3da2a171b3ee1bc1e6196773b570407df6b4")
    }

    #[test]
    fn proof_decode() {
        let proof = Proof::from_raw(&sample_pghr_proof()).unwrap();

        let valid_proof = Proof {
			a: AffineG1::new(
				Fq::from_str("20233418955657178701073640211008243691524800202072436264102260029864396370685").unwrap(),
				Fq::from_str("1928976519703562638864074955635338506310331912135952521920435194862263154244").unwrap(),
			).expect("valid proof.a").into(),
			a_prime: AffineG1::new(
				Fq::from_str("12281512761332781931761325643718907073955505911337440328624942680474388609904").unwrap(),
				Fq::from_str("8424857202475251259707546932458771162956878761447228143749963892065696231681").unwrap(),
			).expect("valid proof.a_prime").into(),
			b: AffineG2::new(
				Fq2::new(
					Fq::from_str("539045453165532223624174985214626049922380177513464865201634923239231964598").unwrap(),
					Fq::from_str("20507014976900324884923703462229212939510025188133599277134408844142237392307").unwrap(),
				),
				Fq2::new(
					Fq::from_str("12178221303165765388438472058234866951705348968791707113588519754197989138595").unwrap(),
					Fq::from_str("17706169778760270831199133527036782630364426470105034702709971627848861923912").unwrap(),
				),
			).expect("valid proof.b").into(),
			b_prime: AffineG1::new(
				Fq::from_str("894143853920341190243212170484394679803393288144322854177056894134935270865").unwrap(),
				Fq::from_str("1789255451122640754682596027209768159190696961108984712413822974704523979347").unwrap(),
			).expect("valid proof.a_prime").into(),
			c: AffineG1::new(
				Fq::from_str("8642719238938976686582714353851496681423252522473326636757666349007357372837").unwrap(),
				Fq::from_str("2580443018084348702991831184281847819675994940179134346784254287297410566354").unwrap(),
			).expect("valid proof.a_prime").into(),
			c_prime: AffineG1::new(
				Fq::from_str("20971419511641251647846544447826190947903620921500049456603446966519708753020").unwrap(),
				Fq::from_str("7739193252819714638863264762490552518044302987179604361042247199869251548216").unwrap(),
			).expect("valid proof.a_prime").into(),
			h: AffineG1::new(
				Fq::from_str("8161024134375723583781487662931037991368404445288561494765687222278251476660").unwrap(),
				Fq::from_str("12145231691119613121869759271688604813008401940544336625017617793883056659758").unwrap(),
			).expect("valid proof.a_prime").into(),
			k: AffineG1::new(
				Fq::from_str("14340527256780616537162338164507243085353277762771788097495866034815839706382").unwrap(),
				Fq::from_str("2679596828065504561135633702093957396311096212464940874032615840876814701035").unwrap(),
			).expect("valid proof.a_prime").into(),
		};

        assert_eq!(proof.a, valid_proof.a,);
        assert_eq!(proof.a_prime, valid_proof.a_prime,);
        assert_eq!(proof.b, valid_proof.b,);
        assert_eq!(proof.b_prime, valid_proof.b_prime,);
        assert_eq!(proof.c, valid_proof.c,);
        assert_eq!(proof.c_prime, valid_proof.c_prime,);
        assert_eq!(proof.k, valid_proof.k,);
        assert_eq!(proof.h, valid_proof.h,);
    }

    #[test]
    fn verification() {
        let vk = vkey();

        let proof = Proof {
			a: AffineG1::new(
				Fq::from_str("20233418955657178701073640211008243691524800202072436264102260029864396370685").unwrap(),
				Fq::from_str("1928976519703562638864074955635338506310331912135952521920435194862263154244").unwrap(),
			).expect("valid proof.a").into(),
			a_prime: AffineG1::new(
				Fq::from_str("12281512761332781931761325643718907073955505911337440328624942680474388609904").unwrap(),
				Fq::from_str("8424857202475251259707546932458771162956878761447228143749963892065696231681").unwrap(),
			).expect("valid proof.a_prime").into(),
			b: AffineG2::new(
				Fq2::new(
					Fq::from_str("539045453165532223624174985214626049922380177513464865201634923239231964598").unwrap(),
					Fq::from_str("20507014976900324884923703462229212939510025188133599277134408844142237392307").unwrap(),
				),
				Fq2::new(
					Fq::from_str("12178221303165765388438472058234866951705348968791707113588519754197989138595").unwrap(),
					Fq::from_str("17706169778760270831199133527036782630364426470105034702709971627848861923912").unwrap(),
				),
			).expect("valid proof.b").into(),
			b_prime: AffineG1::new(
				Fq::from_str("894143853920341190243212170484394679803393288144322854177056894134935270865").unwrap(),
				Fq::from_str("1789255451122640754682596027209768159190696961108984712413822974704523979347").unwrap(),
			).expect("valid proof.a_prime").into(),
			c: AffineG1::new(
				Fq::from_str("8642719238938976686582714353851496681423252522473326636757666349007357372837").unwrap(),
				Fq::from_str("2580443018084348702991831184281847819675994940179134346784254287297410566354").unwrap(),
			).expect("valid proof.a_prime").into(),
			c_prime: AffineG1::new(
				Fq::from_str("20971419511641251647846544447826190947903620921500049456603446966519708753020").unwrap(),
				Fq::from_str("7739193252819714638863264762490552518044302987179604361042247199869251548216").unwrap(),
			).expect("valid proof.a_prime").into(),
			h: AffineG1::new(
				Fq::from_str("8161024134375723583781487662931037991368404445288561494765687222278251476660").unwrap(),
				Fq::from_str("12145231691119613121869759271688604813008401940544336625017617793883056659758").unwrap(),
			).expect("valid proof.a_prime").into(),
			k: AffineG1::new(
				Fq::from_str("14340527256780616537162338164507243085353277762771788097495866034815839706382").unwrap(),
				Fq::from_str("2679596828065504561135633702093957396311096212464940874032615840876814701035").unwrap(),
			).expect("valid proof.a_prime").into(),
		};

        let primary_input = vec![
            Fr::from_str(
                "11893887518801564238850113243068155191401763535822078310914655246254174921707",
            )
            .unwrap(),
            Fr::from_str(
                "9039742628274832857146315176202079824763880684544058044764009859702372701908",
            )
            .unwrap(),
            Fr::from_str(
                "7864248849999267529324215987921491632294157863019983191999113732927809771441",
            )
            .unwrap(),
            Fr::from_str(
                "2886983623257678406932083534975273655277211437585781522465101031866117927530",
            )
            .unwrap(),
            Fr::from_str(
                "1639613592978633992206850322587892881255594351774222883941421746126476816445",
            )
            .unwrap(),
            Fr::from_str(
                "5902043119256669211364401966461491601894820710756687540191805850512824202436",
            )
            .unwrap(),
            Fr::from_str(
                "13692185839566206949758987046107079401517252355870659294323573892338548513162",
            )
            .unwrap(),
            Fr::from_str(
                "213567272714802366240312308317683913515756890632602759628885800370159516315",
            )
            .unwrap(),
            Fr::from_str("170484577853289").unwrap(),
        ];

        assert!(verify(&vk, &primary_input[..], &proof));
    }

    #[test]
    fn verification2() {
        let vk = vkey();
        let proof = Proof {
			a: AffineG1::new(
				Fq::from_str("12873740738727497448187997291915224677121726020054032516825496230827252793177").unwrap(),
				Fq::from_str("21804419174137094775122804775419507726154084057848719988004616848382402162497").unwrap(),
			).expect("valid proof.a").into(),
			a_prime: AffineG1::new(
				Fq::from_str("7742452358972543465462254569134860944739929848367563713587808717088650354556").unwrap(),
				Fq::from_str("7324522103398787664095385319014038380128814213034709026832529060148225837366").unwrap(),
			).expect("valid proof.a_prime").into(),
			b: AffineG2::new(
				Fq2::new(
					Fq::from_str("15588556568726919713003060429893850972163943674590384915350025440408631945055").unwrap(),
					Fq::from_str("8176651290984905087450403379100573157708110416512446269839297438960217797614").unwrap(),
				),
				Fq2::new(
					Fq::from_str("4265071979090628150845437155927259896060451682253086069461962693761322642015").unwrap(),
					Fq::from_str("15347511022514187557142999444367533883366476794364262773195059233657571533367").unwrap(),
				),
			).expect("valid proof.b").into(),
			b_prime: AffineG1::new(
				Fq::from_str("2979746655438963305714517285593753729335852012083057917022078236006592638393").unwrap(),
				Fq::from_str("6470627481646078059765266161088786576504622012540639992486470834383274712950").unwrap(),
			).expect("valid proof.a_prime").into(),
			c: AffineG1::new(
				Fq::from_str("6851077925310461602867742977619883934042581405263014789956638244065803308498").unwrap(),
				Fq::from_str("10336382210592135525880811046708757754106524561907815205241508542912494488506").unwrap(),
			).expect("valid proof.a_prime").into(),
			c_prime: AffineG1::new(
				Fq::from_str("12491625890066296859584468664467427202390981822868257437245835716136010795448").unwrap(),
				Fq::from_str("13818492518017455361318553880921248537817650587494176379915981090396574171686").unwrap(),
			).expect("valid proof.a_prime").into(),
			h: AffineG1::new(
				Fq::from_str("12091046215835229523641173286701717671667447745509192321596954139357866668225").unwrap(),
				Fq::from_str("14446807589950902476683545679847436767890904443411534435294953056557941441758").unwrap(),
			).expect("valid proof.a_prime").into(),
			k: AffineG1::new(
				Fq::from_str("21341087976609916409401737322664290631992568431163400450267978471171152600502").unwrap(),
				Fq::from_str("2942165230690572858696920423896381470344658299915828986338281196715687693170").unwrap(),
			).expect("valid proof.a_prime").into(),
		};

        let primary_input = vec![
            Fr::from_str(
                "13986731495506593864492662381614386532349950841221768152838255933892789078521",
            )
            .unwrap(),
            Fr::from_str(
                "622860516154313070522697309645122400675542217310916019527100517240519630053",
            )
            .unwrap(),
            Fr::from_str(
                "11094488463398718754251685950409355128550342438297986977413505294941943071569",
            )
            .unwrap(),
            Fr::from_str(
                "6627643779954497813586310325594578844876646808666478625705401786271515864467",
            )
            .unwrap(),
            Fr::from_str(
                "2957286918163151606545409668133310005545945782087581890025685458369200827463",
            )
            .unwrap(),
            Fr::from_str(
                "1384290496819542862903939282897996566903332587607290986044945365745128311081",
            )
            .unwrap(),
            Fr::from_str(
                "5613571677741714971687805233468747950848449704454346829971683826953541367271",
            )
            .unwrap(),
            Fr::from_str(
                "9643208548031422463313148630985736896287522941726746581856185889848792022807",
            )
            .unwrap(),
            Fr::from_str("18066496933330839731877828156604").unwrap(),
        ];

        assert!(verify(&vk, &primary_input[..], &proof));
    }
}
