INSERT OR REPLACE INTO folders VALUES(1,'My Folder',1671406935081,1671406935081);
INSERT OR REPLACE INTO folders VALUES(4,'Weekly',1671418617615,1671418617615);
INSERT OR REPLACE INTO folders VALUES(5,'Rustacean',1671447143846,1671447143846);

INSERT OR REPLACE INTO sources VALUES(1,'https://this-week-in-rust.org/rss.xml','Rust',NULL,NULL,1671419394053,1671419394053,0);
INSERT OR REPLACE INTO sources VALUES(2,'https://world.hey.com/this.week.in.rails/feed.atom','Rails',NULL,NULL,1671426476916,1671426476916,0);
INSERT OR REPLACE INTO sources VALUES(3,'https://fasterthanli.me/index.xml','fasterthanli.me',NULL,NULL,1671447888975,1671447888975,0);
INSERT OR REPLACE INTO sources VALUES(4,'https://smallcultfollowing.com/babysteps/atom.xml','Baby Steps',NULL,NULL,1671449269044,1671449269044,0);
INSERT OR REPLACE INTO sources VALUES(5,'https://hoverbear.org/rss.xml','Hoverbear',NULL,NULL,1671449826206,1671449826206,0);
INSERT OR REPLACE INTO sources VALUES(6,'https://jason-williams.co.uk/feed/','Jason Williams',NULL,NULL,1671450381080,1671450381080,0);

INSERT OR REPLACE INTO folder_sources VALUES(4,1);
INSERT OR REPLACE INTO folder_sources VALUES(4,2);
INSERT OR REPLACE INTO folder_sources VALUES(5,3);
INSERT OR REPLACE INTO folder_sources VALUES(5,4);
INSERT OR REPLACE INTO folder_sources VALUES(5,5);
INSERT OR REPLACE INTO folder_sources VALUES(5,6);