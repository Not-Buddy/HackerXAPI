// src/final_challenge.rs
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CityResponse {
    success: bool,
    message: String,
    status: u16,
    data: CityData,
}

#[derive(Deserialize, Debug)]
struct CityData {
    city: String,
}

pub async fn execute_final_challenge() -> Result<String, Box<dyn std::error::Error>> {
    println!("🧠 Starting Sachin's Parallel World Mission...");
    
    // Step 1: Query the Secret City
    println!("🔍 Step 1: Querying secret city...");
    let city_response = get_favorite_city().await?;
    let favorite_city = city_response.data.city;
    println!("📍 Favorite city: {}", favorite_city);
    
    // Step 2: Decode the City using Sachin's travel notes
    println!("🧠 Step 2: Decoding the city landmark...");
    let landmark = decode_city_landmark(&favorite_city);
    println!("🏛️  Landmark in {}: {}", favorite_city, landmark);
    
    // Step 3: Choose Flight Path
    println!("✈️  Step 3: Choosing flight path...");
    let flight_number = get_flight_number(&landmark).await?;
    
    println!("🎯 Flight Number: {}", flight_number);
    println!("🌍 Mission Complete! May the parallel worlds guide your journey home.");
    
    // Return only the flight code
    Ok(flight_number)
}

async fn get_favorite_city() -> Result<CityResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://register.hackrx.in/submissions/myFavouriteCity")
        .send()
        .await?;
    
    let city_response: CityResponse = response.json().await?;
    Ok(city_response)
}

fn decode_city_landmark(city: &str) -> String {
    // Sachin's travel notes mapping from the mission brief
    let mut indian_cities = HashMap::new();
    
    // Indian Cities - Current Location -> Landmark
    indian_cities.insert("Delhi", "Gateway of India");
    indian_cities.insert("Mumbai", "India Gate");
    indian_cities.insert("Chennai", "Charminar");
    indian_cities.insert("Hyderabad", "Taj Mahal");
    indian_cities.insert("Ahmedabad", "Howrah Bridge");
    indian_cities.insert("Mysuru", "Golconda Fort");
    indian_cities.insert("Kochi", "Qutub Minar");
    indian_cities.insert("Pune", "Meenakshi Temple");
    indian_cities.insert("Nagpur", "Lotus Temple");
    indian_cities.insert("Chandigarh", "Mysore Palace");
    indian_cities.insert("Kerala", "Rock Garden");
    indian_cities.insert("Bhopal", "Victoria Memorial");
    indian_cities.insert("Varanasi", "Vidhana Soudha");
    indian_cities.insert("Jaisalmer", "Sun Temple");
    
    // International Cities - Current Location -> Landmark  
    let mut international_cities = HashMap::new();
    international_cities.insert("New York", "Eiffel Tower");
    international_cities.insert("London", "Statue of Liberty");
    international_cities.insert("Tokyo", "Big Ben");
    international_cities.insert("Beijing", "Colosseum");
    international_cities.insert("Bangkok", "Christ the Redeemer");
    international_cities.insert("Toronto", "Burj Khalifa");
    international_cities.insert("Dubai", "CN Tower");
    international_cities.insert("Amsterdam", "Petronas Towers");
    international_cities.insert("Cairo", "Leaning Tower of Pisa");
    international_cities.insert("San Francisco", "Mount Fuji");
    international_cities.insert("Berlin", "Niagara Falls");
    international_cities.insert("Barcelona", "Louvre Museum");
    international_cities.insert("Moscow", "Stonehenge");
    international_cities.insert("Seoul", "Sagrada Familia");
    international_cities.insert("Cape Town", "Acropolis");
    international_cities.insert("Istanbul", "Big Ben");
    international_cities.insert("Riyadh", "Machu Picchu");
    international_cities.insert("Paris", "Taj Mahal");
    international_cities.insert("Dubai Airport", "Moai Statues");
    international_cities.insert("Singapore", "Christchurch Cathedral");
    international_cities.insert("Jakarta", "The Shard");
    international_cities.insert("Vienna", "Blue Mosque");
    international_cities.insert("Kathmandu", "Neuschwanstein Castle");
    international_cities.insert("Los Angeles", "Buckingham Palace");
    international_cities.insert("Mumbai", "Space Needle");
    
    // Check Indian cities first, then international
    if let Some(landmark) = indian_cities.get(city) {
        landmark.to_string()
    } else if let Some(landmark) = international_cities.get(city) {
        landmark.to_string()
    } else {
        "Unknown Landmark".to_string()
    }
}

async fn get_flight_number(landmark: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Choose endpoint based on landmark
    let endpoint = match landmark {
        "Gateway of India" => "https://register.hackrx.in/teams/public/flights/getFirstCityFlightNumber",
        "Taj Mahal" => "https://register.hackrx.in/teams/public/flights/getSecondCityFlightNumber",
        "Eiffel Tower" => "https://register.hackrx.in/teams/public/flights/getThirdCityFlightNumber",
        "Big Ben" => "https://register.hackrx.in/teams/public/flights/getFourthCityFlightNumber",
        _ => "https://register.hackrx.in/teams/public/flights/getFifthCityFlightNumber",
    };
    
    println!("🌐 Calling endpoint: {}", endpoint);
    
    let response = client
        .get(endpoint)
        .send()
        .await?;
    
    let response_text = response.text().await?;
    println!("📥 Response: {}", response_text);
    
    // Extract just the flight code from the response
    extract_flight_code(&response_text)
}

fn extract_flight_code(response: &str) -> Result<String, Box<dyn std::error::Error>> {
    // First try to parse as JSON
    if let Ok(flight_response) = serde_json::from_str::<serde_json::Value>(response) {
        // Check multiple possible JSON structures
        if let Some(flight_num) = flight_response.get("flight_number") {
            return Ok(flight_num.as_str().unwrap_or("UNKNOWN").to_string());
        }
        if let Some(data) = flight_response.get("data") {
            if let Some(flight_num) = data.get("flight_number") {
                return Ok(flight_num.as_str().unwrap_or("UNKNOWN").to_string());
            }
            if let Some(flight_num) = data.get("flightNumber") {
                return Ok(flight_num.as_str().unwrap_or("UNKNOWN").to_string());
            }
        }
        if let Some(flight_num) = flight_response.get("flightNumber") {
            return Ok(flight_num.as_str().unwrap_or("UNKNOWN").to_string());
        }
        if let Some(flight_num) = flight_response.get("flight_code") {
            return Ok(flight_num.as_str().unwrap_or("UNKNOWN").to_string());
        }
        if let Some(flight_num) = flight_response.get("code") {
            return Ok(flight_num.as_str().unwrap_or("UNKNOWN").to_string());
        }
    }
    
    // If JSON parsing fails, try to extract flight code from raw text
    // Look for common flight code patterns (e.g., AI123, 6E456, UK789)
    let flight_code_regex = regex::Regex::new(r"([A-Z]{1,3}\d{3,4})").unwrap();
    if let Some(captures) = flight_code_regex.find(response) {
        return Ok(captures.as_str().to_string());
    }
    
    // Look for quoted strings that might be flight codes
    let quoted_regex = regex::Regex::new(r#""([A-Z0-9]{4,8})""#).unwrap();
    if let Some(captures) = quoted_regex.captures(response) {
        if let Some(flight_code) = captures.get(1) {
            return Ok(flight_code.as_str().to_string());
        }
    }
    
    // If all parsing fails, return the raw response trimmed
    Ok(response.trim().to_string())
}
