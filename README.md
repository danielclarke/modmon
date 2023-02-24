# ModMon
## A moderation monitor for Roblox assets

`modmon --cookie=$cookie --id_file=assets.txt`

`cookie` is the contents of your `.ROBLOSECURITY` file
`id_file` is a list of Roblox asset ids (just the number part), each id on its own line

Modmon will query `https://www.roblox.com/library/{rbxassetid}` for each id and inspect whether the asset is available.

Currently only works for Audio assets.