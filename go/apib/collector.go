package apib

import (
	"fmt"
	"io"
	"sort"
	"sync"
	"time"
)

type Collector struct {
	stopped       bool
	attempts      int32
	successes     int32
	failures      int32
	bytesSent     int64
	bytesReceived int64
	totalLatency  time.Duration
	allLatencies  []time.Duration
	lock          *sync.Mutex
}

func NewCollector() *Collector {
	return &Collector{
		lock: &sync.Mutex{},
	}
}

func (c *Collector) Stop() {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.stopped = true
}

func (c *Collector) Success(start time.Time, bytesSent int, bytesReceived int) bool {
	latency := time.Since(start)
	c.lock.Lock()
	defer c.lock.Unlock()
	c.attempts += 1
	c.successes += 1
	c.bytesSent += int64(bytesSent)
	c.bytesReceived += int64(bytesReceived)
	c.totalLatency += latency
	c.allLatencies = append(c.allLatencies, latency)
	return c.stopped
}

func (c *Collector) Failure() bool {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.attempts += 1
	c.failures += 1
	return c.stopped
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

func (c *Collector) getThroughput(duration time.Duration) float64 {
	return float64(c.successes) / duration.Seconds()
}

func (c *Collector) getAverageLatency() float64 {
	return durationToMillis(c.totalLatency / time.Duration(c.successes))
}

func (c *Collector) getLatencyPercent(durs *durationSort, percent int) float64 {
	ix := (len(c.allLatencies) - 1) * percent / 100
	return durationToMillis(durs.durations[ix])
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
