use gkr::circuit::node::EvalClaim;
use halo2_curves::bn256::Fr;
use halo2_curves::ff::PrimeField;
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

pub fn serialize_fr<S>(fr: &Fr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let bytes = fr.to_repr();
    serializer.serialize_bytes(&bytes)
}

pub fn deserialize_fr<'de, D>(deserializer: D) -> Result<Fr, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: [u8; 32] = <[u8; 32]>::deserialize(deserializer)?;
    let fr = Fr::from_repr(bytes).unwrap();
    Ok(fr)
}

pub fn serialize_fr_vec<S>(fr_vec: &Vec<Fr>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(fr_vec.len()))?;
    for fp in fr_vec {
        seq.serialize_element(&fp.to_bytes())?;
    }
    seq.end()
}

pub fn deserialize_fr_vec<'de, D>(deserializer: D) -> Result<Vec<Fr>, D::Error>
where
    D: Deserializer<'de>,
{
    struct SerializableInputsVisitor;

    impl<'de> Visitor<'de> for SerializableInputsVisitor {
        type Value = SerializationFrVec;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence of byte arrays of length 32")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<SerializationFrVec, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(bytes) = seq.next_element::<[u8; 32]>()? {
                vec.push(Fr::from_bytes(&bytes).expect("Invalid bytes"));
            }
            Ok(SerializationFrVec(vec))
        }
    }

    let serialization_fr_vec = deserializer.deserialize_seq(SerializableInputsVisitor)?;
    Ok(serialization_fr_vec.0)
}

pub fn serialize_eval_claim<S>(eval_claim: &EvalClaim<Fr>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_struct("EvalClaim", 2)?;

    seq.serialize_field("point", &SerializationFrVec(eval_claim.point().to_vec()))?;
    seq.serialize_field("value", &SerializationFr(eval_claim.value()))?;

    seq.end()
}

pub fn deserialize_eval_claim<'de, D>(deserializer: D) -> Result<EvalClaim<Fr>, D::Error>
where
    D: Deserializer<'de>,
{
    // Define the field names that will be deserialized
    const FIELDS: &[&str] = &["point", "value"];

    // Create a visitor to handle deserialization of EvalClaim
    struct EvalClaimVisitor;

    impl<'de> Visitor<'de> for EvalClaimVisitor {
        type Value = EvalClaim<Fr>;

        // Define how the type should be represented when printing debug info
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("struct EvalClaim")
        }

        // Handle deserializing a map (for key-value pairs)
        fn visit_map<V>(self, mut map: V) -> Result<EvalClaim<Fr>, V::Error>
        where
            V: MapAccess<'de>,
        {
            let mut point = None;
            let mut value = None;

            // Iterate through the map and extract fields
            while let Some(key) = map.next_key()? {
                match key {
                    "point" => {
                        if point.is_some() {
                            return Err(de::Error::duplicate_field("point"));
                        }
                        point = Some(map.next_value::<SerializationFrVec>()?.0);
                    }
                    "value" => {
                        if value.is_some() {
                            return Err(de::Error::duplicate_field("value"));
                        }
                        value = Some(map.next_value::<SerializationFr>()?.0);
                    }
                    _ => {
                        return Err(de::Error::unknown_field(key, FIELDS));
                    }
                }
            }

            // Ensure both fields are deserialized
            let point = point.ok_or_else(|| de::Error::missing_field("point"))?;
            let value = value.ok_or_else(|| de::Error::missing_field("value"))?;

            // Return the deserialized EvalClaim
            Ok(EvalClaim::new(point, value))
        }
    }

    // Call deserialize_struct with our visitor
    deserializer.deserialize_struct("EvalClaim", FIELDS, EvalClaimVisitor)
}

pub fn serialize_eval_claims<S>(
    eval_claims: &Vec<EvalClaim<Fr>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(eval_claims.len()))?;
    for claim in eval_claims {
        seq.serialize_element(&SerializationEvalClaim(claim.clone()))?;
    }
    seq.end()
}

pub fn deserialize_eval_claims<'de, D>(deserializer: D) -> Result<Vec<EvalClaim<Fr>>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<SerializationEvalClaim> = Vec::deserialize(deserializer)?;
    let mut eval_claims = Vec::new();
    for claim in vec {
        eval_claims.push(claim.0);
    }
    Ok(eval_claims)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializationFr(
    #[serde(serialize_with = "serialize_fr", deserialize_with = "deserialize_fr")] Fr,
);

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializationFrVec(
    #[serde(
        serialize_with = "serialize_fr_vec",
        deserialize_with = "deserialize_fr_vec"
    )]
    Vec<Fr>,
);

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializationEvalClaim(
    #[serde(
        serialize_with = "serialize_eval_claim",
        deserialize_with = "deserialize_eval_claim"
    )]
    EvalClaim<Fr>,
);

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializationEvalClaims(
    #[serde(
        serialize_with = "serialize_eval_claims",
        deserialize_with = "deserialize_eval_claims"
    )]
    pub Vec<EvalClaim<Fr>>,
);

#[cfg(test)]
pub mod test {
    use gkr::circuit::node::EvalClaim;
    use halo2_curves::bn256::Fr;
    use serde::ser;

    use crate::serialization::{SerializationEvalClaims, SerializationFrVec};

    use super::{SerializationEvalClaim, SerializationFr};

    #[test]
    fn test_serialization_fr() {
        let a = Fr::from(1);
        let wrapper = SerializationFr(a);
        let serialized_f = serde_json::to_string(&wrapper).unwrap();
        let deserialized_f: SerializationFr = serde_json::from_str(&serialized_f).unwrap();

        assert_eq!(a, deserialized_f.0);
    }

    #[test]
    fn test_serialization_fr_vec() {
        let a = Fr::from(1);
        let b = Fr::from(2);
        let fr_vec = vec![a, b];
        let wrapper = SerializationFrVec(fr_vec);
        let serialized_f_vec = serde_json::to_string(&wrapper).unwrap();
        let deserialized_f_vec: SerializationFrVec =
            serde_json::from_str(&serialized_f_vec).unwrap();
        assert_eq!(wrapper.0, deserialized_f_vec.0);
    }

    #[test]
    fn test_serialization_eval_claim() {
        let point = vec![Fr::from(1), Fr::from(2)];
        let value = Fr::from(3);
        let eval_claim = EvalClaim::new(point, value);
        let wrapper = SerializationEvalClaim(eval_claim);
        let serialized_eval_claim = serde_json::to_string(&wrapper).unwrap();
        let deserialized_eval_claim: SerializationEvalClaim =
            serde_json::from_str(&serialized_eval_claim).unwrap();
        assert_eq!(wrapper.0.point(), deserialized_eval_claim.0.point());
        assert_eq!(wrapper.0.value(), deserialized_eval_claim.0.value());
    }

    #[test]
    fn test_serialization_eval_claims() {
        let point = vec![Fr::from(1), Fr::from(2)];
        let value = Fr::from(3);
        let claim = EvalClaim::new(point, value);
        let eval_claims = vec![claim.clone(), claim];
        let wrapper = SerializationEvalClaims(eval_claims);
        let serialized_eval_claims = serde_json::to_string(&wrapper).unwrap();
        let deserialized_eval_claims: SerializationEvalClaims =
            serde_json::from_str(&serialized_eval_claims).unwrap();

        assert_eq!(wrapper.0[0].point(), deserialized_eval_claims.0[0].point());
        assert_eq!(wrapper.0[1].point(), deserialized_eval_claims.0[1].point());
        assert_eq!(wrapper.0[0].value(), deserialized_eval_claims.0[0].value());
        assert_eq!(wrapper.0[1].value(), deserialized_eval_claims.0[1].value());
    }
}
