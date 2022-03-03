use std::io::{Error, Write};

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use rust_rtps_pim::messages::{submessage_elements::Parameter, types::ParameterId};

use crate::{
    mapping_traits::{MappingReadByteOrdered, MappingWriteByteOrdered, NumberOfBytes},
    messages::submessage_elements::{
        ParameterListSubmessageElementRead, ParameterListSubmessageElementWrite, ParameterOwned,
        RtpsParameterBorrowed,
    },
};

const PID_SENTINEL: ParameterId = ParameterId(1);
const SENTINEL: Parameter = Parameter {
    parameter_id: PID_SENTINEL,
    length: 0,
    value: &[],
};

impl<'a> MappingWriteByteOrdered for RtpsParameterBorrowed<'a> {
    fn mapping_write_byte_ordered<W: Write, B: ByteOrder>(
        &self,
        mut writer: W,
    ) -> Result<(), Error> {
        writer.write_u16::<B>(self.parameter_id.0)?;
        writer.write_i16::<B>(self.length)?;
        writer.write_all(self.value.as_ref())?;
        let padding: &[u8] = match self.value.as_ref().len() % 4 {
            1 => &[0; 3],
            2 => &[0; 2],
            3 => &[0; 1],
            _ => &[],
        };
        writer.write_all(padding)?;
        Ok(())
    }
}

impl<'de: 'a, 'a> MappingReadByteOrdered<'de> for RtpsParameterBorrowed<'a> {
    fn mapping_read_byte_ordered<B: ByteOrder>(buf: &mut &'de [u8]) -> Result<Self, Error> {
        let parameter_id = ParameterId(buf.read_u16::<B>()?);
        let length = buf.read_i16::<B>()?;
        let (value, following) = buf.split_at(length as usize);
        *buf = following;
        Ok(Self {
            parameter_id,
            length,
            value: value,
        })
    }
}

impl<'a> NumberOfBytes for RtpsParameterBorrowed<'a> {
    fn number_of_bytes(&self) -> usize {
        4 /* parameter_id and length */ + self.length as usize
    }
}

impl NumberOfBytes for ParameterOwned {
    fn number_of_bytes(&self) -> usize {
        4 /* parameter_id and length */ + self.length as usize
    }
}

impl MappingWriteByteOrdered for ParameterOwned {
    fn mapping_write_byte_ordered<W: Write, B: ByteOrder>(
        &self,
        mut writer: W,
    ) -> Result<(), Error> {
        writer.write_u16::<B>(self.parameter_id.0)?;
        writer.write_i16::<B>(self.length)?;
        writer.write_all(self.value.as_ref())?;
        let padding: &[u8] = match self.value.len() % 4 {
            1 => &[0; 3],
            2 => &[0; 2],
            3 => &[0; 1],
            _ => &[],
        };
        writer.write_all(padding)?;
        Ok(())
    }
}

impl<'a> MappingWriteByteOrdered for ParameterListSubmessageElementWrite<'a> {
    fn mapping_write_byte_ordered<W: Write, B: ByteOrder>(
        &self,
        mut writer: W,
    ) -> Result<(), Error> {
        for parameter in &self.parameter {
            parameter.mapping_write_byte_ordered::<_, B>(&mut writer)?;
        }
        SENTINEL.mapping_write_byte_ordered::<_, B>(&mut writer)
    }
}

impl<'de: 'a, 'a> MappingReadByteOrdered<'de> for ParameterListSubmessageElementRead<'a> {
    fn mapping_read_byte_ordered<B: ByteOrder>(buf: &mut &'de [u8]) -> Result<Self, Error> {
        const MAX_PARAMETERS: usize = 2_usize.pow(16);

        let mut parameter = vec![];

        for _ in 0..MAX_PARAMETERS {
            let parameter_i: Parameter =
                MappingReadByteOrdered::mapping_read_byte_ordered::<B>(buf)?;

            if parameter_i == SENTINEL {
                break;
            } else {
                parameter.push(parameter_i);
            }
        }
        Ok(Self { parameter })
    }
}

impl<'de: 'a, 'a> MappingReadByteOrdered<'de> for Parameter<'a> {
    fn mapping_read_byte_ordered<B: ByteOrder>(buf: &mut &'de [u8]) -> Result<Self, Error> {
        let parameter_id = ParameterId(buf.read_u16::<B>()?);
        let length = buf.read_i16::<B>()?;
        let (value, following) = buf.split_at(length as usize);
        *buf = following;
        Ok(Self {
            parameter_id,
            length,
            value: value,
        })
    }
}

impl<'a> MappingWriteByteOrdered for Parameter<'a> {
    fn mapping_write_byte_ordered<W: Write, B: ByteOrder>(
        &self,
        mut writer: W,
    ) -> Result<(), Error> {
        writer.write_u16::<B>(self.parameter_id.0)?;
        writer.write_i16::<B>(self.length)?;
        writer.write_all(self.value.as_ref())?;
        let padding: &[u8] = match self.value.len() % 4 {
            1 => &[0; 3],
            2 => &[0; 2],
            3 => &[0; 1],
            _ => &[],
        };
        writer.write_all(padding)?;
        Ok(())
    }
}

impl<'a> NumberOfBytes for Parameter<'a> {
    fn number_of_bytes(&self) -> usize {
        4 /* parameter_id and length */ + self.length as usize
    }
}

impl<'a> NumberOfBytes for ParameterListSubmessageElementWrite<'a> {
    fn number_of_bytes(&self) -> usize {
        self.parameter.number_of_bytes() + 4 /* Sentinel */
    }
}

impl<'a> NumberOfBytes for ParameterListSubmessageElementRead<'a> {
    fn number_of_bytes(&self) -> usize {
        self.parameter.number_of_bytes() + 4 /* Sentinel */
    }
}

#[cfg(test)]
mod tests {
    use rust_rtps_pim::messages::submessage_elements::{
        Parameter, ParameterListSubmessageElementConstructor,
    };

    use super::*;
    use crate::mapping_traits::{from_bytes_le, to_bytes_le};

    #[test]
    fn serialize_parameter() {
        let parameter = RtpsParameterBorrowed::new(ParameterId(2), &[5, 6, 7, 8][..]);
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&parameter).unwrap(), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 8,       // value
        ]);
        assert_eq!(parameter.number_of_bytes(), 8)
    }

    #[test]
    fn serialize_parameter_non_multiple_4() {
        let parameter = RtpsParameterBorrowed::new(ParameterId(2), &[5, 6, 7][..]);
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&parameter).unwrap(), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 0,       // value
        ]);
        assert_eq!(parameter.number_of_bytes(), 8)
    }

    #[test]
    fn serialize_parameter_zero_size() {
        let parameter = RtpsParameterBorrowed::new(ParameterId(2), &[][..]);
        assert_eq!(
            to_bytes_le(&parameter).unwrap(),
            vec![
            0x02, 0x00, 0, 0, // Parameter | length
        ]
        );
        assert_eq!(parameter.number_of_bytes(), 4)
    }

    #[test]
    fn deserialize_parameter_non_multiple_of_4() {
        let expected = RtpsParameterBorrowed::new(ParameterId(2), &[5, 6, 7, 8, 9, 10, 11, 0]);
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x02, 0x00, 8, 0, // Parameter | length
            5, 6, 7, 8,       // value
            9, 10, 11, 0,     // value
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter() {
        let expected = RtpsParameterBorrowed::new(ParameterId(2), &[5, 6, 7, 8, 9, 10, 11, 12]);
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x02, 0x00, 8, 0, // Parameter | length
            5, 6, 7, 8,       // value
            9, 10, 11, 12,       // value
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_parameter_list() {
        let parameter_1 = Parameter {
            parameter_id: ParameterId(2),
            length: 4,
            value: &[51, 61, 71, 81],
        };
        let parameter_2 = Parameter {
            parameter_id: ParameterId(3),
            length: 4,
            value: &[52, 62, 0, 0],
        };
        let parameter_list_submessage_element =
            ParameterListSubmessageElementWrite::new(vec![parameter_1, parameter_2]);
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&parameter_list_submessage_element).unwrap(), vec![
            0x02, 0x00, 4, 0, // Parameter ID | length
            51, 61, 71, 81,   // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            52, 62, 0, 0,   // value
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
        assert_eq!(parameter_list_submessage_element.number_of_bytes(), 20);
    }

    #[test]
    fn serialize_parameter_list_empty() {
        let parameter = ParameterListSubmessageElementWrite { parameter: vec![] };
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&parameter).unwrap(), vec![
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
        assert_eq!(parameter.number_of_bytes(), 4);
    }

    #[test]
    fn deserialize_parameter_list() {
        let expected = ParameterListSubmessageElementRead {
            parameter: vec![
                Parameter {
                    parameter_id: ParameterId(0x02),
                    length: 4,
                    value: &[15, 16, 17, 18],
                },
                Parameter {
                    parameter_id: ParameterId(0x03),
                    length: 4,
                    value: &[25, 26, 27, 28],
                },
            ],
        };
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x02, 0x00, 4, 0, // Parameter ID | length
            15, 16, 17, 18,        // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            25, 26, 27, 28,        // value
            0x01, 0x00, 0, 0, // Sentinel: Parameter ID | length
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list_with_long_parameter_including_sentinel() {
        #[rustfmt::skip]
        let parameter_value_expected = &[
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
        ];

        let expected = ParameterListSubmessageElementRead {
            parameter: vec![Parameter {
                parameter_id: ParameterId(0x32),
                length: 24,
                value: parameter_value_expected,
            }],
        };
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list_with_multiple_parameters_with_same_id() {
        #[rustfmt::skip]
        let parameter_value_expected1 = &[
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
        ];
        #[rustfmt::skip]
        let parameter_value_expected2 = &[
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02,
        ];

        let expected = ParameterListSubmessageElementRead {
            parameter: vec![
                Parameter {
                    parameter_id: ParameterId(0x32),
                    length: 24,
                    value: parameter_value_expected1,
                },
                Parameter {
                    parameter_id: ParameterId(0x32),
                    length: 24,
                    value: parameter_value_expected2,
                },
            ],
        };
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]).unwrap();
        assert_eq!(expected, result);
    }
}
