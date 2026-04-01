# Order and Chaos
This repo produces a binary that allows one to play Order and Chaos. The rules of the game are described below. There is also an AI that will play the game with you using alpha-beta pruning.

## Rules
Order and Chaos is similar to tic-tac-toe. The game is played on a 6x6 square grid with X's and O's. One player is Order and the other is Chaos. Order plays first. On each turn, either player may place either an X or an O on any open square. If the Order player is able to make _exactly_ five pieces in a row vertically, horizontally, or diagonally, they win. If Chaos is able to fill the entire board without five pieces in a row, they win. Note that six-in-a-row does _not_ count as a win for Order.

## Playing
Play by running the binary. You will be prompted as to whether you want to play Order or Chaos and whether you want to play with two players or against the AI. On each turn, you will be presented with the current state of the board and may specify a play by giving the coordinates of the piece you wish to place, along with whether you want to play X or O. For example, to play an X at coordinates j2, you would type "j2x".