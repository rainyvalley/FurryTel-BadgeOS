//! The games' rules, tested on the host.  Run with `tools/test-game.sh`.

#[path = "../src/game/mod.rs"]
mod game;
#[path = "../src/record.rs"]
mod record;

mod snake {
    use crate::game::snake::{Dir, SnakeGame, State, Step};

    fn cells(game: &SnakeGame) -> Vec<(u8, u8)> {
        game.segments().collect()
    }

    #[test]
    fn new_game_waits_ready_with_a_three_cell_snake() {
        let game = SnakeGame::new(1);
        assert_eq!(game.state(), State::Ready);
        assert_eq!(game.score(), 0);
        let body = cells(&game);
        assert_eq!(body.len(), 3);
        let mut unique = body.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 3, "no cell repeats");
        assert!(!body.contains(&game.food()), "food starts off the snake");
        assert_eq!(body[0], (10, 4), "head is the east end");
    }

    #[test]
    fn steps_only_while_playing() {
        let mut game = SnakeGame::new(1);
        assert_eq!(game.step(), None);

        game.start();
        game.set_food((0, 0));
        assert_eq!(game.step(), Some(Step::Moved));
        assert_eq!(cells(&game)[0], (11, 4));
    }

    #[test]
    fn eating_grows_the_snake_and_speeds_it_up() {
        let mut game = SnakeGame::new(1);
        let head = cells(&game)[0];
        let slower = game.delay_ms();
        game.set_food((head.0 + 1, head.1));

        game.start();
        assert_eq!(game.step(), Some(Step::Ate));
        assert_eq!(game.score(), 1);
        let body = cells(&game);
        assert_eq!(body.len(), 4);
        assert!(!body.contains(&game.food()), "food respawns off the snake");
        assert!(game.delay_ms() < slower);
    }

    #[test]
    fn grows_along_a_straight_line_without_repeating_cells() {
        let mut game = SnakeGame::new(3);
        game.start();

        let mut eaten = 0;
        for _ in 0..15 {
            let head = cells(&game)[0];
            game.set_food((head.0 + 1, head.1));
            if game.step() != Some(Step::Ate) {
                break;
            }
            eaten += 1;
        }

        assert!(eaten >= 10, "ate {eaten} cells");
        let body = cells(&game);
        assert_eq!(body.len(), 3 + eaten);
        let mut unique = body.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), body.len(), "the ring buffer drops the tail");
    }

    #[test]
    fn walls_end_the_game() {
        let mut game = SnakeGame::new(1);
        game.start();
        game.set_food((0, 0));

        let mut steps = 0;
        while game.step() != Some(Step::Died) {
            steps += 1;
            assert!(steps < 40, "never hit the wall");
        }
        assert_eq!(game.state(), State::Dead);
        assert_eq!(game.step(), None);
    }

    #[test]
    fn cannot_reverse_into_itself() {
        let mut game = SnakeGame::new(1);
        game.start();
        game.set_food((0, 0));

        game.turn(Dir::Left);
        game.step();
        assert_eq!(cells(&game)[0], (11, 4), "kept going right");
    }

    #[test]
    fn only_one_turn_per_step() {
        let mut game = SnakeGame::new(1);
        game.start();
        game.set_food((0, 0));

        game.turn(Dir::Up);
        game.turn(Dir::Left);
        game.step();
        assert_eq!(cells(&game)[0], (10, 3), "the second turn waits");
    }

    #[test]
    fn running_into_itself_ends_the_game() {
        let mut game = SnakeGame::new(1);
        let head = cells(&game)[0];
        game.start();

        game.set_food((head.0 + 1, head.1));
        assert_eq!(game.step(), Some(Step::Ate));
        game.set_food((head.0 + 2, head.1));
        assert_eq!(game.step(), Some(Step::Ate));
        assert_eq!(game.length(), 5);
        game.set_food((0, 0));

        game.turn(Dir::Down);
        assert_eq!(game.step(), Some(Step::Moved));
        game.turn(Dir::Left);
        assert_eq!(game.step(), Some(Step::Moved));
        game.turn(Dir::Up);
        assert_eq!(game.step(), Some(Step::Died));
        assert_eq!(game.state(), State::Dead);
    }

    #[test]
    fn pausing_stops_the_clock() {
        let mut game = SnakeGame::new(1);
        game.start();
        game.pause();
        assert_eq!(game.state(), State::Paused);
        assert_eq!(game.step(), None);
        assert_eq!(cells(&game)[0], (10, 4));

        game.start();
        assert_eq!(game.state(), State::Playing);
        assert_eq!(game.step(), Some(Step::Moved));
    }

    #[test]
    fn food_always_spawns_inside_the_field_and_off_the_snake() {
        let mut game = SnakeGame::new(7);
        for _ in 0..500 {
            game.reset();
            assert!(game.food().0 < crate::game::snake::COLS);
            assert!(game.food().1 < crate::game::snake::ROWS);
            assert!(!cells(&game).contains(&game.food()));
        }
    }
}

mod flappy {
    use crate::game::flappy::{FlappyGame, State};

    #[test]
    fn flapping_lifts_the_bird_then_gravity_wins() {
        let mut game = FlappyGame::new(1);
        game.set_pipes([(300, 5), (400, 5)]);
        game.start();
        game.tick();
        let high = game.bird_y();
        assert!(high < 12, "the flap lifted it from 12, got {high}");

        for _ in 0..20 {
            if game.state() != State::Playing {
                break;
            }
            game.tick();
        }
        assert!(game.bird_y() > high, "gravity pulled it back down");
    }

    #[test]
    fn the_floor_ends_the_game() {
        let mut game = FlappyGame::new(1);
        game.set_pipes([(300, 5), (400, 5)]);
        game.set_y(27);
        game.play();
        game.tick();
        assert_eq!(game.state(), State::Dead);
    }

    #[test]
    fn flying_through_the_gap_is_safe() {
        let mut game = FlappyGame::new(1);
        game.set_y(10);
        game.set_pipes([(30, 4), (300, 4)]);
        game.play();
        game.tick();
        assert_eq!(game.state(), State::Playing);
    }

    #[test]
    fn a_pipe_ends_the_game() {
        let mut game = FlappyGame::new(1);
        game.set_y(24);
        game.set_pipes([(26, 4), (300, 4)]);
        game.play();
        game.tick();
        assert_eq!(game.state(), State::Dead);
    }

    #[test]
    fn passing_a_pipe_scores() {
        let mut game = FlappyGame::new(1);
        game.set_y(10);
        game.set_pipes([(16, 4), (300, 4)]);
        game.play();
        game.tick();
        assert_eq!(game.state(), State::Playing);
        assert_eq!(game.score(), 1);
    }
}

mod crossy {
    use crate::game::crossy::{CrossyGame, Dir, LANES_PER_LEVEL, Lane, State};

    fn grass_everywhere(game: &mut CrossyGame) {
        for row in 0..crate::game::crossy::ROWS {
            game.set_lane(row, Lane::Grass);
        }
    }

    #[test]
    fn hopping_up_scores() {
        let mut game = CrossyGame::new(1);
        grass_everywhere(&mut game);
        game.play();
        assert_eq!(game.row, 7);

        game.hop(Dir::Up);
        assert_eq!(game.row, 6);
        assert_eq!(game.score(), 1);
        assert_eq!(game.state(), State::Playing);
    }

    #[test]
    fn a_car_can_run_the_chicken_over() {
        let mut game = CrossyGame::new(1);
        grass_everywhere(&mut game);
        game.play();
        game.set_player(5, 4);
        game.set_lane(
            4,
            Lane::Road {
                leftward: false,
                period: 1,
                offset: 3,
            },
        );
        assert_eq!(game.state(), State::Playing);

        game.tick();
        assert_eq!(game.state(), State::Dead);
    }

    #[test]
    fn hopping_into_a_car_ends_the_game() {
        let mut game = CrossyGame::new(1);
        grass_everywhere(&mut game);
        game.play();
        game.set_player(5, 6);
        game.set_lane(
            5,
            Lane::Road {
                leftward: false,
                period: 4,
                offset: 5,
            },
        );

        game.hop(Dir::Up);
        assert_eq!(game.state(), State::Dead);
    }

    #[test]
    fn reaching_the_top_scrolls_the_world() {
        let mut game = CrossyGame::new(2);
        grass_everywhere(&mut game);
        game.play();
        game.set_player(10, 0);

        game.hop(Dir::Up);
        assert_eq!(game.row, 0, "the player stays at the top");
        assert_eq!(game.score(), 1);
        assert_eq!(game.state(), State::Playing, "the new lane keeps them safe");
    }

    #[test]
    fn crossing_a_levels_worth_of_lanes_starts_the_next_one() {
        let mut game = CrossyGame::new(4);
        grass_everywhere(&mut game);
        game.play();

        for _ in 0..LANES_PER_LEVEL {
            game.hop(Dir::Up);
        }

        assert_eq!(game.level(), 2);
        assert_eq!(game.row, crate::game::crossy::ROWS - 1, "back at the bottom");
        assert!(game.banner(), "the new level is announced");

        for _ in 0..25 {
            game.tick();
        }
        assert!(!game.banner(), "the banner comes down");
    }

    #[test]
    fn cars_wrap_around_the_edge() {
        let lane = Lane::Road {
            leftward: false,
            period: 2,
            offset: 31,
        };
        assert_eq!(crate::game::crossy::car_start(lane, 0), 31);
        assert!(crate::game::crossy::lane_covers(lane, 31));
        assert!(crate::game::crossy::lane_covers(lane, 0));
        assert!(!crate::game::crossy::lane_covers(lane, 5));
    }

    #[test]
    fn leftward_cars_move_the_other_way() {
        let before = Lane::Road {
            leftward: true,
            period: 1,
            offset: 10,
        };
        let after = Lane::Road {
            leftward: true,
            period: 1,
            offset: 11,
        };
        assert_eq!(
            crate::game::crossy::car_start(before, 0),
            22,
            "offset 10 puts car 0 at 32 - 10"
        );
        assert_eq!(crate::game::crossy::car_start(after, 0), 21);
    }
}


mod record_tests {
    use crate::record::{self, RECORD_MAX};

    #[test]
    fn settings_round_trip() {
        let mut data = record::defaults();
        data.main = [1, 2, 3];
        data.flash = [4, 5, 6];
        data.rainbow = true;
        data.led_mode = 2;
        data.led_levels = [10, 20, 30];
        data.set_note("hello badge");

        let mut buffer = [0; RECORD_MAX];
        let len = record::encode(&data, &mut buffer);
        let decoded = record::decode(&buffer[..len]).expect("decodes");

        assert_eq!(decoded.main, [1, 2, 3]);
        assert_eq!(decoded.flash, [4, 5, 6]);
        assert!(decoded.rainbow);
        assert_eq!(decoded.led_mode, 2);
        assert_eq!(decoded.led_levels, [10, 20, 30]);
        assert_eq!(decoded.note(), "hello badge");
    }

    #[test]
    fn defaults_are_purple_and_red() {
        let data = record::defaults();
        assert_eq!(data.main, [80, 32, 120]);
        assert_eq!(data.flash, [140, 0, 0]);
        assert!(!data.rainbow);
        assert_eq!(data.note(), "");
    }

    #[test]
    fn a_bad_magic_or_checksum_is_rejected() {
        let mut data = record::defaults();
        data.set_note("hi");
        let mut buffer = [0; RECORD_MAX];
        let len = record::encode(&data, &mut buffer);
        assert!(record::decode(&buffer[..len]).is_some());

        let mut bad_magic = buffer;
        bad_magic[0] ^= 0xff;
        assert!(record::decode(&bad_magic[..len]).is_none());

        let mut bad_checksum = buffer;
        bad_checksum[13] ^= 0x01;
        assert!(record::decode(&bad_checksum[..len]).is_none());
    }

    #[test]
    fn a_first_version_record_still_loads() {
        // the old layout had no LED settings: magic, version, rainbow, colors, note, checksum
        let mut buffer = [0u8; RECORD_MAX];
        buffer[0..4].copy_from_slice(&record::MAGIC.to_le_bytes());
        buffer[4] = 1;
        buffer[5] = 0;
        buffer[6..9].copy_from_slice(&[1, 2, 3]);
        buffer[9..12].copy_from_slice(&[4, 5, 6]);
        buffer[12] = 2;
        buffer[13..15].copy_from_slice(b"hi");
        let checksum = buffer[..15]
            .iter()
            .fold(0u32, |sum, &byte| sum.wrapping_add(u32::from(byte)));
        buffer[15..19].copy_from_slice(&checksum.to_le_bytes());

        let decoded = record::decode(&buffer[..19]).expect("decodes");
        assert_eq!(decoded.main, [1, 2, 3]);
        assert_eq!(decoded.flash, [4, 5, 6]);
        assert_eq!(decoded.note(), "hi");
        assert_eq!(decoded.led_mode, 0);
        assert_eq!(decoded.led_levels, record::DEFAULT_LEDS);
    }

    #[test]
    fn a_short_record_is_rejected() {
        assert!(record::decode(&[]).is_none());
        assert!(record::decode(&[0; 12]).is_none());
    }

    #[test]
    fn a_long_note_is_truncated() {
        let mut data = record::defaults();
        data.set_note(&"x".repeat(500));
        assert_eq!(data.note().len(), record::NOTE_LEN);
    }
}

mod tetris_tests {
    use crate::game::tetris::{State, TetrisGame};

    #[test]
    fn the_i_piece_rotates_to_a_column() {
        assert_eq!(TetrisGame::rotate_mask(TetrisGame::piece_mask(0)), 0x4444);
    }

    #[test]
    fn pieces_stay_inside_the_well() {
        let mut game = TetrisGame::new(1);
        game.play();
        game.set_piece(1, 6, 0); // the O piece against the right wall
        assert!(!game.shift(1), "cannot move further right");
        assert!(game.shift(-1), "can move back left");
    }

    #[test]
    fn a_full_row_clears_and_scores() {
        let mut game = TetrisGame::new(1);
        game.play();
        game.set_board_row(23, 0xFF);
        game.set_piece(0, 2, 21); // an I lying across the row above
        game.tick();
        assert_eq!(game.score(), 100);
        assert_eq!(game.board()[23], 0x3C, "the row above dropped into it");
        assert_eq!(game.board()[22], 0);
    }

    #[test]
    fn stacked_full_rows_all_clear() {
        let mut game = TetrisGame::new(1);
        game.play();
        game.set_board_row(22, 0xFF);
        game.set_board_row(23, 0xFF);
        game.set_piece(0, 2, 20); // an I that will lock above them
        game.tick();
        assert_eq!(game.score(), 300, "two rows cleared at once");
        assert_eq!(game.board()[23], 0x3C, "the piece dropped two rows");
        assert_eq!(game.board()[22], 0);
    }

    #[test]
    fn filling_the_well_ends_the_game() {
        let mut game = TetrisGame::new(3);
        game.play();
        game.set_board_row(2, 0xFE);
        game.set_piece(1, 2, 0);
        game.tick();
        assert_eq!(game.state(), State::Dead);
    }

    #[test]
    fn a_hard_drop_lands_on_the_floor() {
        let mut game = TetrisGame::new(1);
        game.play();
        game.set_piece(1, 2, 0); // the O piece
        assert!(game.hard_drop());
        assert_eq!(game.piece().y, 22, "resting on the bottom row");
    }

    #[test]
    fn pausing_stops_gravity() {
        let mut game = TetrisGame::new(1);
        game.play();
        game.pause();
        assert!(!game.tick());
        assert_eq!(game.state(), State::Paused);
    }
}

mod rng_tests {
    use crate::game::rng::Rng;

    #[test]
    fn empty_ranges_do_not_divide_by_zero() {
        let mut rng = Rng::new(1);
        assert_eq!(rng.below(0), 0);
        assert_eq!(rng.range(5, 5), 5);
        assert_eq!(rng.range(9, 3), 9);
    }
}
