    SELECT
	id,
	url,
	title,
	content,
	created,
	updated,
	feed_id,
	(
	    SELECT 
		(CASE count(a.id)
		WHEN 0 THEN NULL
		ELSE json_group_array(
		    json_object(
			'id',
			a.id,
			'name',
			a.name
		    )
		)
		END)
	    FROM
		authors as a
	    JOIN
		article_authors AS aa
	    ON
		aa.a = a.id
	    AND
		aa.t = t.id
	    GROUP BY
		aa.t
	    ORDER BY
		aa.a
	) AS authors
    FROM
	articles AS t
    WHERE
	feed_id = 3
    AND
	updated > 0
    ORDER BY
	updated,
	id
