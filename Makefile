cleandb:
	-rm attendance_data.db*

createdb:
	sqlx database create
	sqlx migrate run

resetdb:
	make cleandb
	make createdb
