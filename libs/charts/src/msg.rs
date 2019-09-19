//! Client/server messages.

use crate::base::*;
use filter::*;

/// Messages from the client to the server.
pub mod to_server {
    use super::*;

    /// Messages from the client to the server.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Msg {
        /// Operations over charts.
        Charts(ChartsMsg),

        /// Operation over filters.
        Filters(FiltersMsg),
    }

    /// Operations over charts.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ChartsMsg {
        /// Creates a new chart.
        New(chart::axis::XAxis, chart::axis::YAxis),
    }
    impl ChartsMsg {
        /// Constructs a chart creation message.
        pub fn new(x: chart::axis::XAxis, y: chart::axis::YAxis) -> Msg {
            Self::New(x, y).into()
        }
    }

    /// Operations over filters.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum FiltersMsg {
        /// Adds a new filter.
        Add {
            /// Filter to add.
            filter: Filter,
        },
        /// Removes a filter.
        Rm {
            /// Index of the filter to remove.
            index: index::Filter,
        },

        /// Operation over a filter.
        Filter {
            /// Index of the filter.
            index: index::Filter,
            /// The operation.
            msg: FilterMsg,
        },
    }

    /// Operations over a filter.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum FilterMsg {
        /// Adds a new sub-filter.
        Add {
            /// Sub-filter to add.
            filter: SubFilter,
        },
        /// Removes a sub-filter.
        Rm {
            /// Index of the sub-filter.
            index: index::SubFilter,
        },
        /// Updates a sub-filter.
        Update {
            /// Index of the sub-filter to replace.
            index: index::SubFilter,
            /// New sub-filter.
            filter: SubFilter,
        },
    }

    impl Into<yew::format::Text> for Msg {
        fn into(self) -> yew::format::Text {
            match self.as_json() {
                Ok(res) => Ok(res),
                Err(e) => failure::bail!("{}", e.pretty()),
            }
        }
    }
    impl Into<yew::format::Binary> for Msg {
        fn into(self) -> yew::format::Binary {
            match self.as_json() {
                Ok(res) => Ok(res.into_bytes()),
                Err(e) => failure::bail!("{}", e.pretty()),
            }
        }
    }

    impl From<FiltersMsg> for Msg {
        fn from(msg: FiltersMsg) -> Self {
            Self::Filters(msg)
        }
    }
    impl From<ChartsMsg> for Msg {
        fn from(msg: ChartsMsg) -> Self {
            Self::Charts(msg)
        }
    }
}

/// Messages from the server to the client.
pub mod to_client {
    use super::*;

    /// Messages from the server to the client.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Msg {
        /// Info about the current allocation data.
        Info,
        /// An alert.
        Alert {
            /// Alert message.
            msg: String,
        },
        /// A message for the charts.
        Charts {
            /// Chart message.
            msg: ChartsMsg,
        },
    }
    impl Msg {
        /// Constructor for `Info`.
        pub fn info() -> Self {
            Self::Info
        }
        /// Constructor for `Alert`.
        pub fn alert<S>(msg: S) -> Self
        where
            S: Into<String>,
        {
            Self::Alert { msg: msg.into() }
        }
        /// Constructor for `Charts`.
        pub fn charts(msg: ChartsMsg) -> Self {
            Self::Charts { msg }
        }
    }

    /// Messages for the charts of the client.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ChartsMsg {
        /// Creates a new chart.
        NewChart(chart::ChartSpec),
        /// Message for a specific chart.
        Chart { uid: uid::ChartUid, msg: ChartMsg },
        /// A new collection of points, overwrites existing points.
        NewPoints(point::ChartPoints),
        /// Some points to append to existing points.
        AddPoints(point::ChartPoints),
    }
    impl ChartsMsg {
        /// Constructor for `NewChart`.
        pub fn new_chart(spec: chart::ChartSpec) -> Msg {
            Msg::charts(Self::NewChart(spec))
        }
        /// Constructor for `NewPoints`.
        pub fn new_points(points: point::ChartPoints) -> Msg {
            Msg::charts(Self::NewPoints(points))
        }
        /// Constructor for `AddPoints`.
        pub fn add_points(points: point::ChartPoints) -> Msg {
            Msg::charts(Self::AddPoints(points))
        }

        /// Constructs a `NewPoints` if `overwrite`, and a `AddPoints` otherwise.
        pub fn points(points: point::ChartPoints, overwrite: bool) -> Msg {
            if overwrite {
                Self::new_points(points)
            } else {
                Self::add_points(points)
            }
        }
    }

    /// Messages for a specific chart in the client.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ChartMsg {
        /// A brand new list of points.
        ///
        /// Replaces all the points in a chart.
        NewPoints(point::Points),
        /// Some points to append.
        Points(point::Points),
    }

    impl ChartMsg {
        /// List of points overwriting the existing points.
        pub fn new_points(uid: uid::ChartUid, points: point::Points) -> Msg {
            Msg::charts(ChartsMsg::Chart {
                uid,
                msg: Self::NewPoints(points),
            })
        }
        /// List of points to append.
        pub fn points(uid: uid::ChartUid, points: point::Points) -> Msg {
            Msg::charts(ChartsMsg::Chart {
                uid,
                msg: Self::Points(points),
            })
        }
    }

    impl Into<Res<Msg>> for RawMsg {
        fn into(self) -> Res<Msg> {
            let res = match self {
                RawMsg::Binary(res_bytes) => {
                    let bytes: Res<Vec<u8>> = res_bytes.map_err(|e| e.into());
                    let bytes = bytes.chain_err(|| "while retrieving message from the server")?;
                    Msg::from_json_bytes(&bytes)
                }
                RawMsg::Text(res_string) => {
                    let string: Res<String> = res_string.map_err(|e| e.into());
                    let string = string.chain_err(|| "while retrieving message from the server")?;
                    Msg::from_json(&string)
                }
            };
            res.chain_err(|| "while parsing a message from the server")
        }
    }

    /// A raw message from the server.
    #[derive(Debug, Clone)]
    pub enum RawMsg {
        /// Binary version.
        Binary(Result<Vec<u8>, String>),
        /// String version.
        Text(Result<String, String>),
    }
    impl From<yew::format::Binary> for RawMsg {
        fn from(data: yew::format::Binary) -> Self {
            RawMsg::Binary(data.map_err(|e| e.to_string()))
        }
    }
    impl From<yew::format::Text> for RawMsg {
        fn from(data: yew::format::Text) -> Self {
            RawMsg::Text(data.map_err(|e| e.to_string()))
        }
    }
}