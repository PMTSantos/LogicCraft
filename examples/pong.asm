; =============================================================================
; LogicCraft — Classic 2-Player Pong in Assembly (`examples/pong.asm`)
; =============================================================================
; Target Architectures: RISC-V RV32I (32-bit) & LogicCraft LC-16 (16-bit)
;
; Memory-Mapped I/O (MMIO) Map:
;   0x8000 .. 0x87FF : VRAM Framebuffer (64x32 resolution, 1 byte per pixel)
;                      Shade 0 = Off, 1 = Dim/Net, 2 = Score/Border, 3 = Bright
;   0xF000           : MMIO_KEY_STATE Bitmask:
;                      Bit 0 (0x01) = Player 1 UP   ('W')
;                      Bit 1 (0x02) = Player 1 DOWN ('S')
;                      Bit 2 (0x04) = Player 2 UP   ('ArrowUp')
;                      Bit 3 (0x08) = Player 2 DOWN ('ArrowDown')
;                      Bit 4 (0x10) = Start / Reset ('Space' / 'Enter')
;   0xF016           : MMIO_VSYNC (Frame Sync strobe)
;   0xF020           : MMIO_P1_SCORE (Player 1 Score)
;   0xF021           : MMIO_P2_SCORE (Player 2 Score)
;   0xF030           : MMIO_VRAM_CLEAR (Hardware fast framebuffer clear)
;
; Register Allocation:
;   s0 : Player 1 Paddle Y (top pixel, 2..24)
;   s1 : Player 2 Paddle Y (top pixel, 2..24)
;   s2 : Ball X (0..63)
;   s3 : Ball Y (1..30)
;   s4 : Ball Velocity dX (-1 or +1)
;   s5 : Ball Velocity dY (-1 or +1)
; =============================================================================

.equ VRAM_BASE,       0x8000
.equ MMIO_BASE,       0xF000
.equ PADDLE_HEIGHT,   6
.equ PADDLE_MIN_Y,    2
.equ PADDLE_MAX_Y,    24
.equ BALL_START_X,    32
.equ BALL_START_Y,    16

.org 0x0000

reset_game:
    li t6, MMIO_BASE
    sb zero, 0x20(t6)        ; Reset Player 1 Score = 0
    sb zero, 0x21(t6)        ; Reset Player 2 Score = 0

    li s0, 13                ; Player 1 Paddle initial Y
    li s1, 13                ; Player 2 Paddle initial Y
    li s2, BALL_START_X      ; Ball initial X = 32
    li s3, BALL_START_Y      ; Ball initial Y = 16
    li s4, 1                 ; Ball initial dX = +1 (moving right)
    li s5, 1                 ; Ball initial dY = +1 (moving down)

game_loop:
    ; -------------------------------------------------------------------------
    ; 1. Read Keyboard Input via MMIO (0xF000)
    ; -------------------------------------------------------------------------
    li t6, MMIO_BASE
    lbu t0, 0(t6)            ; t0 = Key state bitmask

    ; Check Start / Reset key (Space or Enter -> Bit 4 = 0x10)
    andi t1, t0, 16
    bne t1, zero, reset_game

    ; Player 1 UP ('W' -> Bit 0 = 0x01)
    andi t1, t0, 1
    beq t1, zero, check_p1_down
    li t2, PADDLE_MIN_Y
    ble s0, t2, check_p1_down
    addi s0, s0, -1

check_p1_down:
    ; Player 1 DOWN ('S' -> Bit 1 = 0x02)
    andi t1, t0, 2
    beq t1, zero, check_p2_up
    li t2, PADDLE_MAX_Y
    bge s0, t2, check_p2_up
    addi s0, s0, 1

check_p2_up:
    ; Player 2 UP ('ArrowUp' -> Bit 2 = 0x04)
    andi t1, t0, 4
    beq t1, zero, check_p2_down
    li t2, PADDLE_MIN_Y
    ble s1, t2, check_p2_down
    addi s1, s1, -1

check_p2_down:
    ; Player 2 DOWN ('ArrowDown' -> Bit 3 = 0x08)
    andi t1, t0, 8
    beq t1, zero, update_ball
    li t2, PADDLE_MAX_Y
    bge s1, t2, update_ball
    addi s1, s1, 1

    ; -------------------------------------------------------------------------
    ; 2. Ball 2D Movement & Collision Physics
    ; -------------------------------------------------------------------------
update_ball:
    add s2, s2, s4           ; ball_x += dx
    add s3, s3, s5           ; ball_y += dy

    ; Top wall collision (y <= 1) -> dy = -dy
    li t0, 1
    bgt s3, t0, check_bottom_wall
    li s3, 1
    sub s5, zero, s5
    j check_left_paddle

check_bottom_wall:
    ; Bottom wall collision (y >= 30) -> dy = -dy
    li t0, 30
    blt s3, t0, check_left_paddle
    li s3, 30
    sub s5, zero, s5

check_left_paddle:
    ; Check collision with Left Paddle front surface (x == 3 and dx < 0)
    bge s4, zero, check_right_paddle
    li t0, 3
    bne s2, t0, check_left_goal

    ; Check if ball_y is within [p1_y - 1, p1_y + PADDLE_HEIGHT]
    addi t1, s0, -1
    blt s3, t1, check_left_goal
    addi t2, s0, PADDLE_HEIGHT
    bgt s3, t2, check_left_goal

    ; Reflect horizontal velocity (dx = -dx = +1)
    li s4, 1
    li s2, 4
    ; Adjust vertical angle based on paddle hit position
    addi t3, s0, 3
    blt s3, t3, left_hit_top
    li s5, 1
    j render_frame
left_hit_top:
    li s5, -1
    j render_frame

check_right_paddle:
    ; Check collision with Right Paddle front surface (x == 60 and dx > 0)
    li t0, 60
    bne s2, t0, check_right_goal

    ; Check if ball_y is within [p2_y - 1, p2_y + PADDLE_HEIGHT]
    addi t1, s1, -1
    blt s3, t1, check_right_goal
    addi t2, s1, PADDLE_HEIGHT
    bgt s3, t2, check_right_goal

    ; Reflect horizontal velocity (dx = -dx = -1)
    li s4, -1
    li s2, 59
    ; Adjust vertical angle based on paddle hit position
    addi t3, s1, 3
    blt s3, t3, right_hit_top
    li s5, 1
    j render_frame
right_hit_top:
    li s5, -1
    j render_frame

    ; -------------------------------------------------------------------------
    ; 3. Goal / Scoring Detection & Ball Center Reset
    ; -------------------------------------------------------------------------
check_left_goal:
    bgt s2, zero, render_frame
    ; Ball passed Left Goal -> Player 2 Scores!
    lbu t0, 0x21(t6)
    addi t0, t0, 1
    li t1, 9
    ble t0, t1, save_p2_score
    li t0, 0
save_p2_score:
    sb t0, 0x21(t6)
    li s2, BALL_START_X
    li s3, BALL_START_Y
    li s4, 1                 ; Serve toward Player 2
    j render_frame

check_right_goal:
    li t0, 63
    blt s2, t0, render_frame
    ; Ball passed Right Goal -> Player 1 Scores!
    lbu t0, 0x20(t6)
    addi t0, t0, 1
    li t1, 9
    ble t0, t1, save_p1_score
    li t0, 0
save_p1_score:
    sb t0, 0x20(t6)
    li s2, BALL_START_X
    li s3, BALL_START_Y
    li s4, -1                ; Serve toward Player 1

    ; -------------------------------------------------------------------------
    ; 4. Render Scene to Memory-Mapped VRAM (0x8000, 64x32)
    ; -------------------------------------------------------------------------
render_frame:
    li t6, MMIO_BASE
    sb zero, 0x30(t6)        ; Fast-clear VRAM framebuffer (0x8000..0x87FF)

    li t5, VRAM_BASE

    ; 4a. Draw Top & Bottom Court Borders (y = 0 and y = 31, color = 1)
    li t0, 0
    li t1, 64
    li t2, 1
border_loop:
    add t3, t5, t0
    sb t2, 0(t3)             ; Top border pixel (0, x)
    addi t3, t3, 1984        ; Bottom row offset (31 * 64 = 1984)
    sb t2, 0(t3)             ; Bottom border pixel (31, x)
    addi t0, t0, 1
    blt t0, t1, border_loop

    ; 4b. Draw Dashed Center Court Net (x = 32, y = 2, 4, 6..28, color = 1)
    li t0, 2
    li t1, 30
    li t2, 1
net_loop:
    slli t3, t0, 6           ; t3 = y * 64
    addi t3, t3, 32          ; t3 = y * 64 + 32
    add t3, t5, t3
    sb t2, 0(t3)
    addi t0, t0, 2
    blt t0, t1, net_loop

    ; 4c. Draw Player 1 Score Pips (y = 2, x = 28 - i*2, color = 2)
    lbu t0, 0x20(t6)
    li t1, 0
    li t2, 2
    addi t3, t5, 156         ; Base at y = 2, x = 28 (2*64 + 28 = 156)
p1_score_loop:
    bge t1, t0, draw_p2_score
    sb t2, 0(t3)
    sb t2, 64(t3)            ; 2-pixel tall score pip
    addi t3, t3, -2
    addi t1, t1, 1
    j p1_score_loop

draw_p2_score:
    ; 4d. Draw Player 2 Score Pips (y = 2, x = 36 + i*2, color = 2)
    lbu t0, 0x21(t6)
    li t1, 0
    li t2, 2
    addi t3, t5, 164         ; Base at y = 2, x = 36 (2*64 + 36 = 164)
p2_score_loop:
    bge t1, t0, draw_paddles
    sb t2, 0(t3)
    sb t2, 64(t3)            ; 2-pixel tall score pip
    addi t3, t3, 2
    addi t1, t1, 1
    j p2_score_loop

draw_paddles:
    ; 4e. Draw Left Paddle (x = 2, y = s0 .. s0 + PADDLE_HEIGHT - 1, color = 3)
    li t0, 0
    li t1, PADDLE_HEIGHT
    li t2, 3
p1_paddle_loop:
    add t3, s0, t0           ; y = p1_y + i
    slli t3, t3, 6           ; y * 64
    addi t3, t3, 2           ; x = 2
    add t3, t5, t3
    sb t2, 0(t3)
    addi t0, t0, 1
    blt t0, t1, p1_paddle_loop

    ; 4f. Draw Right Paddle (x = 61, y = s1 .. s1 + PADDLE_HEIGHT - 1, color = 3)
    li t0, 0
    li t1, PADDLE_HEIGHT
p2_paddle_loop:
    add t3, s1, t0           ; y = p2_y + i
    slli t3, t3, 6           ; y * 64
    addi t3, t3, 61          ; x = 61
    add t3, t5, t3
    sb t2, 0(t3)
    addi t0, t0, 1
    blt t0, t1, p2_paddle_loop

    ; 4g. Draw Ball at (s2, s3) with bright color = 3
    slli t3, s3, 6           ; ball_y * 64
    add t3, t3, s2           ; + ball_x
    add t3, t5, t3
    sb t2, 0(t3)

    ; -------------------------------------------------------------------------
    ; 5. Frame Pacing / VSync Wait
    ; -------------------------------------------------------------------------
wait_vsync:
    lbu t0, 0x16(t6)         ; Poll MMIO_VSYNC (0xF016)
    beq t0, zero, wait_vsync

    j game_loop
