use failure::*;
use serde::Deserialize;


#[derive(Deserialize,Clone)]
pub struct Release{
    pub id: usize,
    pub draft: bool,
    pub prerelease: bool,
    pub target_commitsh: String,
    pub name: String
}



const RELEASES_URL: &'static str ="https://api.github.com/repos/PhilipK/BoilR/releases";

//https://api.github.com/repos/PhilipK/BoilR/releases/65177266/assets


pub async fn fetch_newest_release() -> Option<Release>{
    if let Some(response) = reqwest::get(RELEASES_URL).await{
        if let Some(releases) = response.json::<Vec<Release>>().await{
            return releases.iter().filter(|r| !r.draft && !r.prerelease).next();
        }
    }
    None
}


//Plan
//On start, fetch releases
//First one that is not draft and not prelease, check its target commit
//Compare with current commit
//If different, show a popup warning of new version
//User clicks upgrade
//find name of current program
//download to same location
//open new instance
//shut down old
