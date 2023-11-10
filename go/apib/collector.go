package apib

import (
	"fmt"
	"io"
	"sort"
	"sync"
	"sync/atomic"
	"time"
)

type Collector struct {
	stopped           atomic.Bool
	intervalSuccesses atomic.Uint32
	intervalFailures  atomic.Uint32
	attempts          uint32
	successes         uint32
	failures          uint32
	bytesSent         uint64
	bytesReceived     uint64
	lastError         atomic.Pointer[error]
	totalLatency      time.Duration
	allLatencies      []time.Duration
	lock              *sync.Mutex
}

func NewCollector() *Collector {
	return &Collector{
		lock: &sync.Mutex{},
	}
}

func (c *Collector) Stop() {
	c.stopped.Store(true)
}

func (c *Collector) Success() bool {
	c.intervalSuccesses.Add(1)
	return c.stopped.Load()
}

func (c *Collector) Failure(err error) bool {
	c.intervalFailures.Add(1)
	c.lastError.Store(&err)
	return c.stopped.Load()
}

func (c *Collector) Collect(stats *LocalCollector) {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.attempts += stats.attempts
	c.successes += stats.successes
	c.failures += stats.failures
	c.bytesSent += stats.bytesSent
	c.bytesReceived += stats.bytesReceived
	c.totalLatency += stats.totalLatency
	c.allLatencies = append(c.allLatencies, stats.allLatencies...)
}

func (c *Collector) Write(start time.Time, end time.Time, out io.Writer) {
	duration := end.Sub(start)
	c.lock.Lock()
	defer c.lock.Unlock()
	durs := &durationSort{c.allLatencies}
	sort.Sort(durs)

	fmt.Fprintf(out, "Duration:           %.3f\n", duration.Seconds())
	fmt.Fprintf(out, "Attempted requests: %d\n", c.attempts)
	fmt.Fprintf(out, "Sucessful requests: %d\n", c.successes)
	fmt.Fprintf(out, "Errors:             %d\n", c.failures)
	fmt.Fprintf(out, "\n")
	fmt.Fprintf(out, "Throughput:       %.3f requests/second\n", c.getThroughput(duration))
	fmt.Fprintf(out, "Average Latency:  %.3f milliseconds\n", c.getAverageLatency())
	fmt.Fprintf(out, "Minimum Latency:  %.3f milliseconds\n", c.getLatencyPercent(durs, 0))
	fmt.Fprintf(out, "Maximum Latency:  %.3f milliseconds\n", c.getLatencyPercent(durs, 100))
	fmt.Fprintf(out, "50%% Latency:      %.3f milliseconds\n", c.getLatencyPercent(durs, 50))
	fmt.Fprintf(out, "90%% Latency:      %.3f milliseconds\n", c.getLatencyPercent(durs, 90))
	fmt.Fprintf(out, "95%% Latency:      %.3f milliseconds\n", c.getLatencyPercent(durs, 95))
	fmt.Fprintf(out, "99%% Latency:      %.3f milliseconds\n", c.getLatencyPercent(durs, 99))
}

func (c *Collector) WriteTick(start time.Time, tickStart time.Time, testDuration time.Duration, out io.Writer) time.Time {
	now := time.Now()
	soFar := now.Sub(start) / time.Second
	intervalDuration := now.Sub(tickStart)

	intervalSuccesses := c.intervalSuccesses.Swap(0)
	intervalFailures := c.intervalFailures.Swap(0)
	lastError := c.lastError.Swap(nil)
	throughput := float64(intervalSuccesses) / intervalDuration.Seconds()

	if intervalFailures > 0 {
		fmt.Fprintf(out, "(%d / %d) %.3f (%d errors)\n", soFar, testDuration/time.Second, throughput, intervalFailures)
		if lastError != nil {
			fmt.Fprintf(out, "  %v\n", lastError)
		}
	} else {
		fmt.Fprintf(out, "(%d / %d) %.3f\n", soFar, testDuration/time.Second, throughput)
	}
	return now
}

func (c *Collector) getThroughput(duration time.Duration) float64 {
	return float64(c.successes) / duration.Seconds()
}

func (c *Collector) getAverageLatency() float64 {
	if c.successes == 0 {
		return 0.0
	}
	return durationToMillis(c.totalLatency / time.Duration(c.successes))
}

func (c *Collector) getLatencyPercent(durs *durationSort, percent int) float64 {
	if len(c.allLatencies) == 0 {
		return 0.0
	}
	ix := (len(c.allLatencies) - 1) * percent / 100
	return durationToMillis(durs.durations[ix])
}

type LocalCollector struct {
	attempts      uint32
	successes     uint32
	failures      uint32
	bytesSent     uint64
	bytesReceived uint64
	totalLatency  time.Duration
	allLatencies  []time.Duration
}

func NewLocalCollector() *LocalCollector {
	return &LocalCollector{}
}

func (c *LocalCollector) Success(start time.Time, bytesSent int, bytesReceived int) {
	latency := time.Since(start)
	c.attempts += 1
	c.successes += 1
	c.bytesSent += uint64(bytesSent)
	c.bytesReceived += uint64(bytesReceived)
	c.totalLatency += latency
	c.allLatencies = append(c.allLatencies, latency)
}

func (c *LocalCollector) Failure() {
	c.attempts += 1
	c.failures += 1
}

func durationToMillis(d time.Duration) float64 {
	return float64(d) / 1000000.0
}

type durationSort struct {
	durations []time.Duration
}

func (s *durationSort) Len() int {
	return len(s.durations)
}

func (s *durationSort) Less(i, j int) bool {
	return s.durations[i] < s.durations[j]
}

func (s *durationSort) Swap(i, j int) {
	tmp := s.durations[i]
	s.durations[i] = s.durations[j]
	s.durations[j] = tmp
}
