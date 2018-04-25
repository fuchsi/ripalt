/*
 * ripalt
 * Copyright (C) 2018 Daniel Müller
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use super::*;

use std::sync::mpsc;
use std::thread;

use chrono::Duration;

use db::DbExecutor;
use schema::peers;

const CLEANUP_INTERVAL: u64 = 60;
const SLEEP_PER_LOOP: u64 = 2;

pub fn cleanup(dbe: DbExecutor, rx: &mpsc::Receiver<bool>) {
    info!("started cleanup thread");

    loop {
        // delete stale peers older than 60 minutes
        let date = Utc::now()
            .checked_sub_signed(Duration::minutes(60))
            .unwrap();
        let db: &PgConnection = &dbe.conn();
        let res = diesel::delete(peers::table)
            .filter(peers::dsl::updated_at.lt(date))
            .execute(db);

        match res {
            Ok(num) => debug!("deleted {} orphaned peers", num),
            Err(e) => warn!("error while cleaning orphaned peers: {}", e),
        }

        let mut count: u64 = CLEANUP_INTERVAL;
        while count > 0 {
            // try to receive from the main_rx in order to terminate
            if rx.try_recv().is_ok() {
                info!("shutting down cleanup thread");
                return;
            }

            thread::sleep(std::time::Duration::from_secs(SLEEP_PER_LOOP));
            count -= SLEEP_PER_LOOP;
        }
    }
}
