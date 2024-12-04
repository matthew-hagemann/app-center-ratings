use crate::{
    conn,
    db::{Category, Timeframe, VoteSummary},
    proto::{
        chart::{
            chart_server::{self, ChartServer},
            ChartData as PbChartData, GetChartRequest, GetChartResponse,
        },
        common::{Rating as PbRating, RatingsBand as PbRatingsBand},
    },
    ratings::{Chart, ChartData, Rating, RatingsBand},
};
use cached::{proc_macro::cached, Return};
use tonic::{Request, Response, Status};
use tracing::{error, info};

#[derive(Copy, Clone, Debug)]
pub struct ChartService;

impl ChartService {
    pub fn new_server() -> ChartServer<ChartService> {
        ChartServer::new(ChartService)
    }
}

#[tonic::async_trait]
impl chart_server::Chart for ChartService {
    async fn get_chart(
        &self,
        request: Request<GetChartRequest>,
    ) -> Result<Response<GetChartResponse>, Status> {
        let GetChartRequest {
            timeframe,
            category,
        } = request.into_inner();

        let category = match category {
            Some(c) => Some(
                Category::from_repr(c).ok_or(Status::invalid_argument("invalid category value"))?,
            ),
            None => None,
        };

        let timeframe = Timeframe::from_repr(timeframe).unwrap_or(Timeframe::Unspecified);

        let chart = get_chart_cached(category, timeframe).await;

        match chart {
            Ok(chart) if chart.data.is_empty() => {
                Err(Status::not_found("Cannot find data for given timeframe."))
            }

            Ok(chart) => {
                if chart.was_cached {
                    info!(
                        "Using cached chart data for category '{:?}' in timeframe '{:?}'",
                        category, timeframe
                    );
                }

                let ordered_chart_data = chart.value.data.into_iter().map(|cd| cd.into()).collect();

                let payload = GetChartResponse {
                    timeframe: timeframe as i32,
                    category: category.map(|c| c as i32),
                    ordered_chart_data,
                };

                Ok(Response::new(payload))
            }

            Err(e) => {
                error!("unable to fetch vote summary: {e}");
                Err(Status::unknown("Internal server error"))
            }
        }
    }
}

#[cached(
    time = 86400, // 24 hours
    sync_writes = true,
    result = true,
    with_cached_flag = true
)]
async fn get_chart_cached(
    category: Option<Category>,
    timeframe: Timeframe,
) -> Result<Return<Chart>, Box<dyn std::error::Error>> {
    let summaries = VoteSummary::get_for_timeframe(timeframe, category, conn!()).await?;

    Ok(Return::new(Chart::new(timeframe, summaries)))
}

impl From<ChartData> for PbChartData {
    fn from(value: ChartData) -> Self {
        Self {
            raw_rating: value.raw_rating,
            rating: Some(value.rating.into()),
        }
    }
}

impl From<Rating> for PbRating {
    fn from(r: Rating) -> Self {
        Self {
            snap_id: r.snap_id,
            total_votes: r.total_votes,
            ratings_band: r.ratings_band as i32,
        }
    }
}

impl From<PbRating> for Rating {
    fn from(r: PbRating) -> Self {
        Self {
            snap_id: r.snap_id,
            total_votes: r.total_votes,
            ratings_band: RatingsBand::from_repr(r.ratings_band).unwrap(),
        }
    }
}

impl From<RatingsBand> for PbRatingsBand {
    fn from(rb: RatingsBand) -> Self {
        match rb {
            RatingsBand::VeryGood => Self::VeryGood,
            RatingsBand::Good => Self::Good,
            RatingsBand::Neutral => Self::Neutral,
            RatingsBand::Poor => Self::Poor,
            RatingsBand::VeryPoor => Self::VeryPoor,
            RatingsBand::InsufficientVotes => Self::InsufficientVotes,
        }
    }
}
