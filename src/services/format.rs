//! Value formatting for explicitly supported report locales and currencies.
use crate::repository::{DataValue, Error, Result};

#[derive(Clone, Copy)]
pub(super) struct FormatContext<'a> {
    pub locale: &'a str,
    pub currency: &'a str,
}
pub(super) fn value(value: &DataValue, style: &str, context: FormatContext<'_>) -> Result<String> {
    if !matches!(style, "text" | "number" | "currency" | "percent" | "date") {
        return Err(Error::InvalidReport(format!(
            "unknown value format: {style}"
        )));
    }
    match value {
        DataValue::Integer(number) if matches!(style, "number" | "currency" | "percent") => {
            numeric(&number.to_string(), *number < 0, style, context)
        }
        DataValue::Decimal(number) if matches!(style, "number" | "currency" | "percent") => {
            numeric(
                &number.abs().to_string(),
                number.is_sign_negative(),
                style,
                context,
            )
        }
        DataValue::Text(text) | DataValue::Date(text) if style == "date" => {
            date(text, context.locale)
        }
        _ => Ok(value.display()),
    }
}
fn numeric(input: &str, negative: bool, style: &str, context: FormatContext<'_>) -> Result<String> {
    if !matches!(context.locale, "en-US" | "es-MX") {
        return Err(Error::Unsupported(format!(
            "numeric locale {}",
            context.locale
        )));
    }
    let (prefix, precision) = if style == "currency" {
        currency(context.locale, context.currency)?
    } else {
        (String::new(), 2)
    };
    let shift = if style == "percent" { 2_i32 } else { 0 };
    let unsigned = input.trim_start_matches('-');
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((m, e)) => (
            m,
            e.parse::<i32>()
                .map_err(|_| Error::InvalidData("invalid decimal exponent".into()))?,
        ),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{integer}{fraction}");
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidData("invalid number".into()));
    }
    let position = integer.len() as i32 + exponent + shift;
    if !(-1024..=1024).contains(&position) {
        return Err(Error::InvalidData("number is out of range".into()));
    }
    let (whole, frac) = if position <= 0 {
        (
            "0".to_owned(),
            format!("{}{}", "0".repeat((-position) as usize), digits),
        )
    } else if position as usize >= digits.len() {
        (
            format!("{}{}", digits, "0".repeat(position as usize - digits.len())),
            String::new(),
        )
    } else {
        (
            digits[..position as usize].to_owned(),
            digits[position as usize..].to_owned(),
        )
    };
    let mut raw = format!(
        "{}{}",
        whole,
        frac.chars().take(precision).collect::<String>()
    )
    .into_bytes();
    if frac.len() < precision {
        raw.extend(std::iter::repeat_n(b'0', precision - frac.len()));
    }
    if frac.as_bytes().get(precision).is_some_and(|b| *b >= b'5') {
        let mut carry = true;
        for digit in raw.iter_mut().rev() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            raw.insert(0, b'1');
        }
    }
    let split = raw.len() - precision;
    let mut whole = std::str::from_utf8(&raw[..split])
        .expect("digits")
        .trim_start_matches('0');
    if whole.is_empty() {
        whole = "0";
    }
    let grouped = whole
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut output, (index, ch)| {
            if index > 0 && index % 3 == 0 {
                output.push(',');
            }
            output.push(ch);
            output
        })
        .chars()
        .rev()
        .collect::<String>();
    let mut fraction = std::str::from_utf8(&raw[split..])
        .expect("digits")
        .to_owned();
    if style != "currency" {
        while fraction.ends_with('0') {
            fraction.pop();
        }
    }
    let number = if fraction.is_empty() {
        grouped
    } else {
        format!("{grouped}.{fraction}")
    };
    let sign = if negative { "-" } else { "" };
    Ok(format!(
        "{sign}{prefix}{number}{}",
        if style == "percent" { "%" } else { "" }
    ))
}
fn currency(locale: &str, code: &str) -> Result<(String, usize)> {
    let precision = match code {
        "USD" | "MXN" | "EUR" | "CAD" | "GBP" => 2,
        "JPY" => 0,
        _ => return Err(Error::Unsupported(format!("currency {code}"))),
    };
    let prefix = match (locale, code) {
        ("en-US", "USD") => "$",
        ("en-US", "MXN") => "MX$",
        ("en-US", "EUR") => "€",
        ("en-US", "CAD") => "CA$",
        ("en-US", "GBP") => "£",
        ("en-US", "JPY") => "¥",
        ("es-MX", "MXN") => "$",
        ("es-MX", "USD") => "USD\u{a0}",
        ("es-MX", "EUR") => "EUR\u{a0}",
        ("es-MX", "CAD") => "CAD\u{a0}",
        ("es-MX", "GBP") => "GBP\u{a0}",
        ("es-MX", "JPY") => "JPY\u{a0}",
        _ => return Err(Error::Unsupported(format!("currency {code} in {locale}"))),
    };
    Ok((prefix.to_owned(), precision))
}
fn date(value: &str, locale: &str) -> Result<String> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Ok(value.into());
    }
    let year = value[0..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..10].parse::<u32>().unwrap_or(0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days {
        return Ok(value.into());
    }
    match locale {
        "en-US" => Ok(format!("{month}/{day}/{year}")),
        "es-MX" => Ok(format!("{day}/{month}/{year}")),
        _ => Err(Error::Unsupported(format!("date locale {locale}"))),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const US: FormatContext<'static> = FormatContext {
        locale: "en-US",
        currency: "USD",
    };
    #[test]
    fn rounding_and_grouping() {
        for (v, want) in [
            (1.005, "1.01"),
            (-1.005, "-1.01"),
            (2.675, "2.68"),
            (999.995, "1,000"),
            (-0.0001, "-0"),
            (12345.675, "12,345.68"),
        ] {
            assert_eq!(value(&DataValue::Decimal(v), "number", US).unwrap(), want);
        }
    }
    #[test]
    fn money_percent_and_yen() {
        assert_eq!(
            value(&DataValue::Decimal(1500.25), "currency", US).unwrap(),
            "$1,500.25"
        );
        assert_eq!(
            value(&DataValue::Decimal(0.12567), "percent", US).unwrap(),
            "12.57%"
        );
        assert_eq!(
            value(
                &DataValue::Integer(100),
                "currency",
                FormatContext {
                    locale: "en-US",
                    currency: "JPY"
                }
            )
            .unwrap(),
            "¥100"
        );
    }
    #[test]
    fn mexico_foreign_currency() {
        assert_eq!(
            value(
                &DataValue::Integer(1234),
                "currency",
                FormatContext {
                    locale: "es-MX",
                    currency: "USD"
                }
            )
            .unwrap(),
            "USD\u{a0}1,234.00"
        );
        assert_eq!(
            value(
                &DataValue::Integer(1234),
                "currency",
                FormatContext {
                    locale: "es-MX",
                    currency: "MXN"
                }
            )
            .unwrap(),
            "$1,234.00"
        );
    }
    #[test]
    fn date_locales_and_invalid_date() {
        assert_eq!(
            value(&DataValue::Text("2026-01-02".into()), "date", US).unwrap(),
            "1/2/2026"
        );
        assert_eq!(
            value(
                &DataValue::Text("2026-01-02".into()),
                "date",
                FormatContext {
                    locale: "es-MX",
                    currency: "MXN"
                }
            )
            .unwrap(),
            "2/1/2026"
        );
        assert_eq!(
            value(&DataValue::Text("2025-02-29".into()), "date", US).unwrap(),
            "2025-02-29"
        );
    }
    #[test]
    fn unsupported_locale_is_explicit() {
        assert!(matches!(
            value(
                &DataValue::Integer(1),
                "number",
                FormatContext {
                    locale: "fr-FR",
                    currency: "EUR"
                }
            ),
            Err(Error::Unsupported(_))
        ));
    }
    #[test]
    fn matches_designer_intl_reference_cases() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/intl-number-cases.json"))
                .unwrap();
        for case in cases.as_array().unwrap() {
            let locale = case["locale"].as_str().unwrap();
            let currency = case["currency"].as_str().unwrap();
            let style = case["style"].as_str().unwrap();
            let input = case["input"].as_str().unwrap();
            let got = value(
                &DataValue::Decimal(input.parse::<f64>().unwrap()),
                style,
                FormatContext { locale, currency },
            )
            .unwrap();
            assert_eq!(
                got,
                case["expected"].as_str().unwrap(),
                "locale={locale} style={style} currency={currency} input={input}"
            );
        }
    }
}
