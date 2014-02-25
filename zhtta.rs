//
// zhtta.rs
//
// Starting code for PS3
// Running on Rust 0.9
//
// Note that this code has serious security risks!	You should not run it 
// on any system with access to sensitive files.
// 
// University of Virginia - cs4414 Spring 2014
// Weilin Xu and David Evans
// Version 0.5

// To see debug! outputs set the RUST_LOG environment variable, e.g.: export RUST_LOG="zhtta=debug"

#[feature(globs)];
extern mod extra;

use std::io::*;
use std::io::net::ip::{SocketAddr};
use std::{os, str, libc, from_str};
use std::path::Path;
use std::hashmap::HashMap;
use extra::priority_queue::PriorityQueue;
use std::cmp;

use extra::getopts;
use extra::arc::MutexArc;

use gash::*;
mod gash;

static SERVER_NAME : &'static str = "Zhtta Version 0.5";

static IP : &'static str = "127.0.0.1";
static PORT : uint = 4414;
static WWW_DIR : &'static str = "./www";

static HTTP_OK : &'static str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n";
static HTTP_BAD : &'static str = "HTTP/1.1 404 Not Found\r\n\r\n";

static VIRGINIA_IP1_PREFIX : &'static str = "128.143.";
static VIRGINIA_IP2_PREFIX : &'static str = "137.54.";
static LOCALHOST_IP : &'static str = "127.0.0.1";

static COUNTER_STYLE : &'static str = "<doctype !html><html><head><title>Hello, Rust!</title>
			 <style>body { background-color: #884414; color: #FFEEAA}
					h1 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm red }
					h2 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm green }
			 </style></head>
			 <body>";

struct HTTP_Request {
	// Use peer_name as the key to access TcpStream in hashmap. 

	// (Due to a bug in extra::arc in Rust 0.9, it is very inconvenient to use TcpStream without the "Freeze" bound.
	//	See issue: https://github.com/mozilla/rust/issues/12139)
	peer_name: ~SocketAddr,
	path: ~Path,
}

impl HTTP_Request {
	/* Calculates priority for a request. Implements SPTF and location prioritization
		by settings the priority of a request equal to its file size, and adding 
		10,000,000,000 to the priority of any non-Charlottesville request.
		This guarantees that any Charlottesville requests still have priority,
		unless someone tries to get a file > 10GB from the server...
	*/
	fn get_priority(&self) -> u64 {
		let fsize = self.path.stat().size;
		if (self.peer_name.ip.to_str().starts_with(VIRGINIA_IP1_PREFIX) ||
			self.peer_name.ip.to_str().starts_with(VIRGINIA_IP2_PREFIX) ||
			self.peer_name.ip.to_str().starts_with(LOCALHOST_IP))
		{ 20000000000 - fsize} else { 20000000000 - fsize }
	}
}

impl cmp::Ord for HTTP_Request {
	fn lt(&self, other: &HTTP_Request) -> bool {
		let myPriority = self.get_priority();
		let otherPriority = other.get_priority();
		myPriority < otherPriority
	}
}

struct WebServer {
	ip: ~str,
	port: uint,
	www_dir_path: ~Path,
	
	request_queue_arc: MutexArc<PriorityQueue<HTTP_Request>>,
	visitor_count_arc: MutexArc<uint>,
	// Hashes on string instead of SocketAddr (just cuz for hashing)
	stream_map_arc: MutexArc<HashMap<~str, Option<std::io::net::tcp::TcpStream>>>,
	
	notify_port: Port<()>,
	shared_notify_chan: SharedChan<()>,
}

impl WebServer {
	fn new(ip: &str, port: uint, www_dir: &str) -> WebServer {
		let (notify_port, shared_notify_chan) = SharedChan::new();
		let www_dir_path = ~Path::new(www_dir);
		os::change_dir(www_dir_path.clone());

		WebServer {
			ip: ip.to_owned(),
			port: port,
			www_dir_path: www_dir_path,
						
			request_queue_arc: MutexArc::new(PriorityQueue::new()),
			visitor_count_arc: MutexArc::new(0),
			stream_map_arc: MutexArc::new(HashMap::new()),
			
			notify_port: notify_port,
			shared_notify_chan: shared_notify_chan,		   
		}
	}
	
	fn run(&mut self) {
		self.listen();
		self.dequeue_static_file_request();
	}
	
	fn listen(&mut self) {
		let addr = from_str::<SocketAddr>(format!("{:s}:{:u}", self.ip, self.port)).expect("Address error.");
		let www_dir_path_str = self.www_dir_path.as_str().expect("invalid www path?").to_owned();
		
		let request_queue_arc = self.request_queue_arc.clone();
		let visitor_count_arc = self.visitor_count_arc.clone();
		let shared_notify_chan = self.shared_notify_chan.clone();
		let stream_map_arc = self.stream_map_arc.clone();
				
		spawn(proc() {
			let mut acceptor = net::tcp::TcpListener::bind(addr).listen();
			println!("{:s} listening on {:s} (serving from: {:s}).", 
					 SERVER_NAME, addr.to_str(), www_dir_path_str);
			
			for stream in acceptor.incoming() {
				let (queue_port, queue_chan) = Chan::new();
				queue_chan.send(request_queue_arc.clone());

				let (visit_count_port, visit_count_chan) = Chan::new();
				visit_count_chan.send(visitor_count_arc.clone());
				
				let notify_chan = shared_notify_chan.clone();
				let stream_map_arc = stream_map_arc.clone();
				
				// Spawn a task to handle the connection.
				spawn(proc() {
					let request_queue_arc = queue_port.recv();
					
					let visitor_count_arc = visit_count_port.recv();
					visitor_count_arc.access( |count| { *count +=  1; });
				  
					let mut stream = stream;
					
					let peer_name = WebServer::get_peer_name(&mut stream);
					
					let mut buf = [0, ..500];
					stream.read(buf);
					let request_str = str::from_utf8(buf);
					debug!("Request:\n{:s}", request_str);
					
					let req_group : ~[&str]= request_str.splitn(' ', 3).collect();
					if req_group.len() > 2 {
						let path_str = "." + req_group[1].to_owned();
						
						let mut path_obj = ~os::getcwd();
						path_obj.push(path_str.clone());
						
						let ext_str = match path_obj.extension_str() {
							Some(e) => e,
							None => "",
						};
						
						debug!("Requested path: [{:s}]", path_obj.as_str().expect("error"));
						debug!("Requested path: [{:s}]", path_str);
							 
						if path_str == ~"./" {
							debug!("===== Counter Page request =====");
							WebServer::respond_with_counter_page(stream, visitor_count_arc.access( |count| { return *count; } ) );
							debug!("=====Terminated connection from [{:s}].=====", peer_name.to_str());
						} else if !path_obj.exists() || path_obj.is_dir() {
							debug!("===== Error page request =====");
							WebServer::respond_with_error_page(stream, path_obj);
							debug!("=====Terminated connection from [{:s}].=====", peer_name.to_str());
						} else if ext_str == "shtml" { // Dynamic web pages.
							debug!("===== Dynamic Page request =====");
							WebServer::respond_with_dynamic_page(stream, path_obj);
							debug!("=====Terminated connection from [{:s}].=====", peer_name.to_str());
						} else { 
							debug!("===== Static Page request =====");
							WebServer::enqueue_static_file_request(stream, path_obj, stream_map_arc, request_queue_arc, notify_chan);
						}
					}
				});
			}
		});
	}

	fn respond_with_error_page(stream: Option<std::io::net::tcp::TcpStream>, path: &Path) {
		let mut stream = stream;
		let msg: ~str = format!("Cannot open: {:s}", path.as_str().expect("invalid path").to_owned());

		stream.write(HTTP_BAD.as_bytes());
		stream.write(msg.as_bytes());
	}

	// DONE: Safe visitor counter.
	fn respond_with_counter_page(stream: Option<std::io::net::tcp::TcpStream>, visit_count : uint) {
		let mut stream = stream;
		let response: ~str = 
			format!("{:s}{:s}<h1>Greetings, Krusty!</h1>
					 <h2>Visitor count: {:u}</h2></body></html>\r\n", 
					HTTP_OK, COUNTER_STYLE, 
					visit_count );
		debug!("Responding to counter request");
		stream.write(response.as_bytes());
	}
	
	// DONE: Streaming file.
	// TODO: Application-layer file caching.
	fn respond_with_static_file(stream: Option<std::io::net::tcp::TcpStream>, path: &Path) {
		let mut file_reader = File::open(path).expect("Invalid file!");
		let (write_port, write_chan) = Chan::new();
		let chunk_size : uint = 1000000;
		let num_chunks = path.stat().size / (chunk_size as u64);

		spawn(proc() {
			let mut stream = stream;
			stream.write(HTTP_OK.as_bytes());
			for i in range (0, num_chunks) {
				let chunk : ~[u8] = write_port.recv();
				stream.write(chunk);
			}
			stream.write(write_port.recv());
		});
		
		for i in range (0, num_chunks) {
			write_chan.send(file_reader.read_bytes(chunk_size));
		}
		write_chan.send(file_reader.read_to_end());
		// This sends last bit if that is < size divider.
		/*	NOTE: This seems to make it slower? Sending 256 bytes at once took 735 seconds (compared to like 15 normally).
			Sending 1024 bytes at once took 154 seconds.
			Sending 1000000 bytes at once took 12 seconds, so faster...? Neat.
		*/

		//stream.write(file_reader.read_to_end());
	}
	
	// DONE: Server-side gashing.
	fn respond_with_dynamic_page(stream: Option<std::io::net::tcp::TcpStream>, path: &Path) {
		// for now, just serve as static file
		let mut stream = stream;
		let mut file = File::open(path).expect("Invalid file!");	// Open file.
		let fileBuf = file.read_to_end();							// Read to buffer.
		let mut fileStr = str::from_utf8(fileBuf).to_owned();		// Convert buffer to string.
		let mut commandIndex = match fileStr.find_str("<!--#exec cmd=\"") { Some(x) => x, None => -1 };
			// Find index of first command prefix.
		while(commandIndex != -1) {
			let fileStrCopy = fileStr; // To avoid issues with fileStr being borrowed.
			let splitOnCommand = [fileStrCopy.slice_to(commandIndex), fileStrCopy.slice_from(commandIndex+15)];
				// splitOnCommand[0] = Beginning of file before command prefix.
				// splitOnCommand[1] = Command + suffix + rest of file. 15 is length of command prefix, remove that.
			let endCommandIndex = match splitOnCommand[1].find_str("\" -->") { Some(x) => x, None => -1 };
				// Index of command suffix.
			let splitOnEndCommand = [splitOnCommand[1].slice_to(endCommandIndex), splitOnCommand[1].slice_from(endCommandIndex+5)];
				// splitOnEndCommand[0] = Command.
				// splitOnEndCommand[1] = Rest of file, after command suffix. 5 is length of command suffix, remove that.
			let cmdResult = gash::run_cmdline(splitOnEndCommand[0]); // Actual value of command result.
			fileStr = (splitOnCommand[0] + cmdResult + splitOnEndCommand[1]);
				// Replace command prefix + command + command suffix in fileStr with command result.
			commandIndex = match fileStr.find_str("<!--#exec cmd=\"") { Some(x) => x, None => -1 };
				// Get next command prefix, restart. Will continue until there are no more commands.
		}
		stream.write(HTTP_OK.as_bytes());
		stream.write(fileStr.as_bytes());
	}
	
	// DONE: Smarter Scheduling.
	fn enqueue_static_file_request(stream: Option<std::io::net::tcp::TcpStream>, path_obj: &Path, stream_map_arc: MutexArc<HashMap<~str, Option<std::io::net::tcp::TcpStream>>>, req_queue_arc: MutexArc<PriorityQueue<HTTP_Request>>, notify_chan: SharedChan<()>) {
		// Save stream in hashmap for later response.
		let mut stream = stream;
		let peer_name = WebServer::get_peer_name(&mut stream);
		let (stream_port, stream_chan) = Chan::new();
		stream_chan.send(stream);
		unsafe {
			// Use an unsafe method, because TcpStream in Rust 0.9 doesn't have "Freeze" bound.
			stream_map_arc.unsafe_access(|local_stream_map| {
				let stream = stream_port.recv();
				local_stream_map.swap(peer_name.clone().to_str(), stream);
			});
		}

		// Enqueue the HTTP request.
		let req = HTTP_Request { peer_name: peer_name.clone(), path: ~path_obj.clone() };
		let (req_port, req_chan) = Chan::new();
		req_chan.send(req);

		debug!("Waiting for queue mutex lock.");
		req_queue_arc.access(|local_req_queue| {
			debug!("Got queue mutex lock.");
			let req: HTTP_Request = req_port.recv();
			local_req_queue.push(req);
			debug!("A new request enqueued, now the length of queue is {:u}.", local_req_queue.len());
		});
		
		notify_chan.send(()); // Send incoming notification to responder task.
	
	
	}

	// DONE: Smarter Scheduling.
	fn dequeue_static_file_request(&mut self) {
		// Port<> cannot be sent to another task. So we have to make this task as the main task that can access self.notify_port.
		let mut handler_comms: ~[Chan<()>] = ~[];
		let (handler_finished_port, handler_finished_chan) = SharedChan::new();

		// Benchmarking tests
		// --- Before : 1.851s, 1.820s, 1.862s, 1.989s, 1.857s => avg: 1.876
		// --- 1 task : 1.832s, 1.868s, 1.861s, 1.864s, 1.852s => avg: 1.855
		// --- 2 tasks: 1.602s, 1.585s, 1.611s, 1.605s, 1.583s => avg: 1.597
		// --- 3 tasks: 1.565s, 1.583s, 1.589s, 1.583s, 1.573s => avg: 1.579
		// --- 4 tasks: 1.578s, 1.582s, 1.596s, 1.582s, 1.591s => avg: 1.586
		// --- 7 tasks: 1.579s, 1.588s, 1.602s, 1.622s, 1.574s => avg: 1.593
		// --- 8 tasks: 1.614s, 1.593s, 1.598s, 1.588s, 1.593s => avg: 1.597
		// --- 16 task: 1.565s, 1.592s, 1.586s, 1.580s, 1.591s => avg: 1.583
		// --- 32 task: 1.586s, 1.613s, 1.631s, 1.584s, 1.600s => avg: 1.603
		// All tests done immediately after Step 4 (multiple response tasks),
		// not accurate times at/after Step 5 (SPTF) and beyond.
		for i in range(0, 4) {
			let (handler_port, handler_chan) = Chan::new();
			let req_queue_get = self.request_queue_arc.clone();
			let stream_map_get = self.stream_map_arc.clone();
			let handler_finished_chan_clone = handler_finished_chan.clone();
			handler_comms.push(handler_chan);
			spawn(proc() { WebServer::static_file_request_handler(i, req_queue_get, stream_map_get, handler_port, handler_finished_chan_clone); });
		}

		loop {
			self.notify_port.recv();	// waiting for new request enqueued.
			let handler_index = handler_finished_port.recv();
			handler_comms[handler_index].send(());
		}
	}

	fn static_file_request_handler(i: int, req_queue_get: MutexArc<PriorityQueue<HTTP_Request>>, stream_map_get: MutexArc<HashMap<~str, Option<std::io::net::tcp::TcpStream>>>, handler_port: Port<()>, handler_finished_chan: SharedChan<int>) {
		let (request_port, request_chan) = Chan::new();
		loop {
			handler_finished_chan.send(i);
			//println!("Port {:d} is ready!", i);
			handler_port.recv();
			req_queue_get.access( |req_queue| {
				match req_queue.maybe_pop() { // FIFO queue.
					None => { /* do nothing */ }
					Some(req) => {
						// println!("My file size is {:u}, my IP is {:s}, my priority is {:u}", req.path.stat().size, req.peer_name.ip.to_str(), req.get_priority())
						// ^ Was using that to test that SPTF worked. It did!
						/* NOTE FOR US TO TALK ABOUT:
							It does the first n first no matter the size, where n is the for loop bound above (we set to 4).
							Then it's pretty good about doing smaller ones first, but it's hard to tell on the zhtta-test-NUL.txt
							file... It is apparent on zhtta-test2-NUL.txt
						*/
						request_chan.send(req);
						debug!("A new request dequeued, now the length of queue is {:u}.", req_queue.len());
					}
				}
			});
			
			let request = request_port.recv();
			
			// Get stream from hashmap.
			// Use unsafe method, because TcpStream in Rust 0.9 doesn't have "Freeze" bound.
			let (stream_port, stream_chan) = Chan::new();
			unsafe {
				stream_map_get.unsafe_access(|local_stream_map| {
					let stream = local_stream_map.pop(&request.peer_name.to_str()).expect("no option tcpstream");
					stream_chan.send(stream);
				});
			}
			
			// DONE: Spawning more tasks to respond the dequeued requests concurrently. You may need a semophore to control the concurrency.
			// ^ We did the above in dequeue_static_file_request instead of static_file_request_handler, still should behave the same though.
			let stream = stream_port.recv();
			WebServer::respond_with_static_file(stream, request.path);
			// Close stream automatically.
			debug!("=====Terminated connection from [{:s}].=====", request.peer_name.to_str());
		}
	}
	
	fn get_peer_name(stream: &mut Option<std::io::net::tcp::TcpStream>) -> ~SocketAddr {
		let default : Option<SocketAddr> = FromStr::from_str(IP + ":" + PORT.to_str());
		match *stream {
			Some(ref mut s) => {
						 match s.peer_name() {
							Some(pn) => {~pn},
							None => (~default.unwrap())
						 }
					   },
			None => (~default.unwrap())
		}
	}
}

fn get_args() -> (~str, uint, ~str) {
	fn print_usage(program: &str) {
		println!("Usage: {:s} [options]", program);
		println!("--ip	   \tIP address, \"{:s}\" by default.", IP);
		println!("--port   \tport number, \"{:u}\" by default.", PORT);
		println!("--www	\tworking directory, \"{:s}\" by default", WWW_DIR);
		println("-h --help \tUsage");
	}
	
	/* Begin processing program arguments and initiate the parameters. */
	let args = os::args();
	let program = args[0].clone();
	
	let opts = ~[
		getopts::optopt("ip"),
		getopts::optopt("port"),
		getopts::optopt("www"),
		getopts::optflag("h"),
		getopts::optflag("help")
	];

	let matches = match getopts::getopts(args.tail(), opts) {
		Ok(m) => { m }
		Err(f) => { fail!(f.to_err_msg()) }
	};

	if matches.opt_present("h") || matches.opt_present("help") {
		print_usage(program);
		unsafe { libc::exit(1); }
	}
	
	let ip_str = if matches.opt_present("ip") {
					matches.opt_str("ip").expect("invalid ip address?").to_owned()
				 } else {
					IP.to_owned()
				 };
	
	let port:uint = if matches.opt_present("port") {
						from_str::from_str(matches.opt_str("port").expect("invalid port number?")).expect("not uint?")
					} else {
						PORT
					};
	
	let www_dir_str = if matches.opt_present("www") {
						matches.opt_str("www").expect("invalid www argument?") 
					  } else { WWW_DIR.to_owned() };
	
	(ip_str, port, www_dir_str)
}

fn main() {
	let (ip_str, port, www_dir_str) = get_args();
	let mut zhtta = WebServer::new(ip_str, port, www_dir_str);
	zhtta.run();
}
